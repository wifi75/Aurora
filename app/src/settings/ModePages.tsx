import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo, useState, type ReactNode } from 'react';
import { getControls, getDigi, getSat, putDigi, putSat, satRefreshTle, type DigiSettings, type SatSettings } from '../api/client';
import { ClusterOptionsFields } from '../components/digi/ClusterOptionsDialog';
import { MacroEditor } from '../components/digi/MacroEditor';
import { ModeSettingsFields } from '../components/digi/ModeSettings';
import { SatSettingsPane, type SatCategory } from '../components/sat/SatSettingsPane';
import { WsprExtras } from '../components/WsprExtras';
import { useT } from '../i18n';
import { useClusterOptions } from '../lib/clusterOptions';
import { digiWithDefaults, editMacros, type MacroScope } from '../lib/digiMacroEdit';
import { cssColor, effectiveMacros, EVENT_MACROS, keyNames, textColorFor, type Macro } from '../lib/macroLang';
import { openInMain } from '../lib/sdr';
import { toast } from '../lib/toast';
import { WsprnetGroup } from './OnlineTab';
import { SequenceTab } from './SequenceTab';
import { Actions, Group, Message, Page } from './layout';

/** I riquadri dei componenti presi dai dialoghi delle pagine dei modi diventano riquadri delle Impostazioni. */
const box = (title: string, children: ReactNode) => (
  <Group key={title} title={title}>
    <div className="sw-fields">{children}</div>
  </Group>
);

const DIGI_NAMES: Record<DigiSettings['mode'], string> = { psk31: 'PSK31', psk63: 'PSK63', rtty: 'RTTY', cw: 'CW' };

/** Le impostazioni di PSK, RTTY e CW lette dal motore, con il salvataggio di una modifica (subito, come nella pagina). */
function useDigiSettings() {
  const t = useT();
  const client = useQueryClient();
  const query = useQuery({ queryKey: ['digi'], queryFn: getDigi, retry: false });
  const snap = useMemo(() => (query.data ? digiWithDefaults(query.data) : null), [query.data]);
  const save = (patch: Partial<DigiSettings>) => {
    if (!snap) return;
    void putDigi({ ...snap.settings, ...patch }).then(
      (next) => client.setQueryData(['digi'], next),
      () => toast(t('digi.saveError'), 'error'),
    );
  };
  return { query, snap, save };
}

/** Modi › PSK · RTTY · CW › Ricezione e trasmissione: il contenuto di «Impostazioni del modo» senza i dati personali (stanno in Stazione). */
export function DigiModePage() {
  const t = useT();
  const { query, snap, save } = useDigiSettings();
  if (query.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!snap) return <p className="muted">{t('sw.digi.loading')}</p>;
  return (
    <Page>
      <ModeSettingsFields settings={snap.settings} levelDb={null} onSave={save} allModes section={box} />
    </Page>
  );
}

/** Modi › PSK · RTTY · CW › Macro: l'elenco delle macro del modo in uso, ognuna si apre nell'editor della pagina. */
export function DigiMacrosPage() {
  const t = useT();
  const { query, snap, save } = useDigiSettings();
  const [editing, setEditing] = useState<{ name: string; scope: MacroScope } | null>(null);
  if (query.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!snap) return <p className="muted">{t('sw.digi.loading')}</p>;
  const s = snap.settings;
  const toMacro = (m: { name: string; label: string; text: string; color: number | null }): Macro => ({ name: m.name, label: m.label, text: m.text, color: m.color });
  const macros = effectiveMacros(s.macros.map(toMacro), s.mode_macros[s.mode]?.map(toMacro), s.merge_macros[s.mode] !== false);
  const order = [...keyNames(), ...EVENT_MACROS];
  const shown = order.filter((name) => {
    const m = macros.get(name);
    return m !== undefined && (m.label.trim() !== '' || m.text.trim() !== '');
  });
  const scopeOf = (name: string): MacroScope => ((s.mode_macros[s.mode] ?? []).some((m) => m.name === name) ? 'mode' : 'all');
  const firstFree = keyNames().find((n) => !shown.includes(n)) ?? 'F1';
  const edited = editing ? (macros.get(editing.name) ?? { name: editing.name, label: '', text: '', color: null }) : null;
  return (
    <Page>
      <Group title={t('sw.pg.digiMacros')} span={3}>
        <Message kind="muted">{t('sw.macros.mode', { mode: DIGI_NAMES[s.mode] })}</Message>
        {shown.length === 0 ? <Message kind="muted">{t('sw.macros.empty')}</Message> : null}
        <ul className="sw-macros">
          {shown.map((name) => {
            const m = macros.get(name);
            if (!m) return null;
            const bg = cssColor(m.color);
            return (
              <li key={name}>
                <b className="sw-macro-key" style={bg ? { background: bg, color: textColorFor(m.color) } : undefined}>
                  {name}
                </b>
                <span className="sw-macro-label">{m.label || m.text.slice(0, 40)}</span>
                <small className="muted">{scopeOf(name) === 'mode' ? t('digi.edit.scopeMode', { mode: DIGI_NAMES[s.mode] }) : t('digi.edit.scopeAll')}</small>
                <button type="button" className="glass" onClick={() => setEditing({ name, scope: scopeOf(name) })}>
                  {t('sw.macros.edit')}
                </button>
              </li>
            );
          })}
        </ul>
        <Actions>
          <button type="button" className="glass" onClick={() => setEditing({ name: firstFree, scope: 'all' })}>
            {t('sw.macros.new')}
          </button>
        </Actions>
      </Group>
      {editing && edited ? (
        <MacroEditor
          key={editing.name}
          macro={edited}
          modeName={DIGI_NAMES[s.mode]}
          scope={editing.scope}
          onSave={(m, where) => {
            save(editMacros(s, editing.name, m, where));
            setEditing(null);
          }}
          onCancel={() => setEditing(null)}
        />
      ) : null}
    </Page>
  );
}

/** Modi › PSK · RTTY · CW › Opzioni del DX Cluster: le stesse della rotella del cluster nella pagina. */
export function DigiClusterPage() {
  const [options, setOptions] = useClusterOptions();
  return (
    <Page>
      <ClusterOptionsFields options={options} onChange={setOptions} section={box} />
    </Page>
  );
}

/** Modi › Satelliti: una pagina per categoria (orbite, mappa, Doppler e rotore), gli stessi contenuti del vecchio dialogo. */
export function SatSettingsPage({ cat }: { cat: SatCategory }) {
  const t = useT();
  const client = useQueryClient();
  const query = useQuery({ queryKey: ['sat'], queryFn: getSat, retry: false });
  const [busy, setBusy] = useState(false);
  if (query.isError) return <p className="notice">{t('set.loadError')}</p>;
  const snap = query.data;
  if (!snap) return <p className="muted">{t('set.loading')}</p>;
  const save = (patch: Partial<SatSettings>) => {
    void putSat({ ...snap.settings, ...patch }).then((next) => client.setQueryData(['sat'], next));
  };
  const refresh = () => {
    setBusy(true);
    satRefreshTle()
      .then(
        (n) => {
          toast(t('sat.tle.done', { n }), 'ok');
          void client.invalidateQueries({ queryKey: ['sat'] });
        },
        (e: unknown) => toast(t('sat.tle.error', { why: e instanceof Error ? e.message : String(e) }), 'error'),
      )
      .finally(() => setBusy(false));
  };
  return (
    <Page>
      <Group title={cat === 'orbits' ? t('sw.pg.orbits') : cat === 'map' ? t('sat.set.catMap') : t('sw.pg.doppler')} span={3}>
        <div className="sw-fields">
          <SatSettingsPane cat={cat} snap={snap} busy={busy} onSave={save} onRefresh={refresh} />
        </div>
      </Group>
    </Page>
  );
}

/** Modi › WSPR · FST4W: potenza del radiofaro, Tx Pct e Tipo 1 (gli stessi della barra di Live) e lo spot su WSPRnet. */
export function WsprPage() {
  const t = useT();
  // i valori del radiofaro arrivano con gli interruttori del motore: nella finestra delle Impostazioni nessun altro li legge
  useQuery({ queryKey: ['controls'], queryFn: getControls, refetchInterval: 3000, retry: false });
  return (
    <Page>
      <Group title={t('sw.wspr.beacon')} span={3}>
        <div className="sw-wspr">
          <WsprExtras />
        </div>
      </Group>
      <WsprnetGroup />
    </Page>
  );
}

/** Modi › Contest · Fox/Hound: la verifica del Fox (Hound) e la strada per le impostazioni di ogni contest, che stanno nella loro pagina. */
export function ContestSettingsPage() {
  const t = useT();
  return (
    <div className="sw-page">
      <SequenceTab part="fox" />
      <Page>
        <Group title={t('sw.contest.title')}>
          <Message kind="muted">{t('sw.contest.note')}</Message>
          <Actions>
            <button type="button" className="glass" onClick={() => openInMain('#/contest')}>
              {t('sw.contest.open')}
            </button>
          </Actions>
        </Group>
      </Page>
    </div>
  );
}
