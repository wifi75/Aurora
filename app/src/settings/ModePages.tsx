import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo, useState, type ReactNode } from 'react';
import { getContest } from '../api/contest';
import { getControls, getDigi, getSat, putDigi, putSat, satRefreshTle, type DigiSettings, type SatSettings } from '../api/client';
import { ClusterOptionsFields } from '../components/digi/ClusterOptionsDialog';
import { MacroEditor } from '../components/digi/MacroEditor';
import { ModeSettingsFields } from '../components/digi/ModeSettings';
import { SatSettingsPane, type SatCategory } from '../components/sat/SatSettingsPane';
import { WsprExtras } from '../components/WsprExtras';
import { useT } from '../i18n';
import { useClusterOptions } from '../lib/clusterOptions';
import { ContestSettingsForm } from '../pages/ContestPage';
import { digiWithDefaults, editMacros, type MacroScope } from '../lib/digiMacroEdit';
import { clearMemory, memorySummary, MEMORY_SLOTS, readMemories, readScan, writeScan, type ScanProps } from '../lib/digiScan';
import { cssColor, effectiveMacros, EVENT_MACROS, keyNames, textColorFor, type Macro } from '../lib/macroLang';
import { openInMain } from '../lib/sdr';
import { browserStorage } from '../lib/theme';
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

/** Modi › PSK · RTTY · CW › Scansione e memorie: le proprieta' della scansione e le 20 memorie del menu Modo, da vedere, richiamare e svuotare. */
export function DigiScanPage() {
  const t = useT();
  const { snap, save } = useDigiSettings();
  const [scan, setScan] = useState<ScanProps>(() => readScan(browserStorage()));
  const [mems, setMems] = useState(() => readMemories(browserStorage()));
  const setProp = (patch: Partial<ScanProps>) => {
    const next = { ...scan, ...patch };
    setScan(next);
    writeScan(browserStorage(), next);
  };
  const slots = Array.from({ length: MEMORY_SLOTS }, (_, i) => i + 1);
  return (
    <Page>
      <Group title={t('digi.scan.title')}>
        <div className="sw-fields">
          <label>
            <span>{t('digi.scan.dwell')}</span>
            <input type="number" min={2} max={600} value={scan.dwell} onChange={(e) => setProp({ dwell: Math.min(600, Math.max(2, Number(e.target.value) || 10)) })} />
          </label>
          {(['once', 'timeMarks', 'freqMarks'] as const).map((k) => (
            <label key={k} className="sstv-check">
              <input type="checkbox" checked={scan[k]} onChange={(e) => setProp({ [k]: e.target.checked })} />
              <span>{t(`digi.scan.${k}` as 'digi.scan.once')}</span>
            </label>
          ))}
          <p className="sstv-hint">{t('sw.scan.hint')}</p>
        </div>
      </Group>
      <Group title={t('sw.mem.title')} span={2}>
        <Message kind="muted">{t('sw.mem.hint')}</Message>
        <ul className="sw-mems">
          {slots.map((n) => {
            const m = mems[String(n)];
            return (
              <li key={n}>
                <b>{t('digi.mi.slot', { n })}</b>
                {m ? <span>{memorySummary(m)}</span> : <span className="sw-mem-empty">{t('sw.mem.empty')}</span>}
                <button
                  type="button"
                  className="glass"
                  disabled={!m || !snap}
                  onClick={() => {
                    if (!m) return;
                    save(m);
                    toast(t('digi.run.memLoaded', { n }), 'ok');
                  }}
                >
                  {t('sw.mem.load')}
                </button>
                <button
                  type="button"
                  className="glass"
                  disabled={!m}
                  onClick={() => {
                    clearMemory(browserStorage(), n);
                    setMems(readMemories(browserStorage()));
                  }}
                >
                  {t('sw.mem.clear')}
                </button>
              </li>
            );
          })}
        </ul>
      </Group>
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

/**
 * Modi › Contest · Fox/Hound: la verifica del Fox (Hound) e le impostazioni di un contest, le stesse della finestrella della pagina Contest.
 * Si apre sul contest attivo; senza, si sceglie dall'elenco quale preparare.
 */
export function ContestSettingsPage() {
  const t = useT();
  const client = useQueryClient();
  const stato = useQuery({ queryKey: ['contest'], queryFn: getContest, retry: false });
  const [scelto, setScelto] = useState('');
  const elenco = stato.data?.contest ?? [];
  const id = scelto || stato.data?.attivo || '';
  return (
    <div className="sw-page">
      <SequenceTab part="fox" />
      <Page>
        <Group title={t('sw.contest.title')} span={3}>
          {stato.isError ? <Message kind="alert">{t('contest.unavailable')}</Message> : null}
          {elenco.length ? (
            <label className="sw-contest-pick">
              <span>{t('sw.contest.which')}</span>
              <select value={id} onChange={(e) => setScelto(e.target.value)}>
                {id ? null : <option value="">{t('sw.contest.choose')}</option>}
                {elenco.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.nome}
                    {c.id === stato.data?.attivo ? ` · ${t('sw.contest.active')}` : ''}
                  </option>
                ))}
              </select>
            </label>
          ) : null}
          {id ? (
            <div className="sw-contest">
              <ContestSettingsForm key={id} id={id} t={t} onSaved={() => void client.invalidateQueries({ queryKey: ['contest'] })} />
            </div>
          ) : (
            <Message kind="muted">{t('sw.contest.note')}</Message>
          )}
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
