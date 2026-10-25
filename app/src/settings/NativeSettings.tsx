import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { ApiError, getEngineSettings, getStationInfo, putEngineSettings } from '../api/client';
import type { EngineSettings } from '../api/schemas';
import { useT, type Key, type T } from '../i18n';
import { ENGINE_GROUPS, type EngineGroup } from '../lib/engineGroups';
import { REPORT_GROUPS, type SettingRow } from '../lib/engineSettingsLayout';
import { Help } from './common';
import { Cells, Group, Notes, Page, Unavailable } from './layout';

/** Vero quando Aurora gira con il motore proprio: le impostazioni sono sue, non di un altro programma. */
export function useIsNative(): boolean | undefined {
  const info = useQuery({ queryKey: ['station-info'], queryFn: getStationInfo, retry: false });
  if (info.isError) return false;
  return info.data ? info.data.engine === 'native' : undefined;
}

export function useSettings() {
  return useQuery({ queryKey: ['engine-settings'], queryFn: getEngineSettings, retry: false });
}

function errorText(error: unknown, t: T): string {
  if (error instanceof ApiError) {
    if (error.code === 'local_only') return t('ens.err.local');
    if (error.code === 'bad_setting') return t('ens.err.bad');
  }
  return t('ens.err.failed');
}

export interface Draft {
  changes: Record<string, string>;
  set(key: string, value: string, current: string | undefined): void;
}

export function useDraft(): Draft & { clear(): void } {
  const [changes, setChanges] = useState<Record<string, string>>({});
  return {
    changes,
    set(key, value, current) {
      setChanges((previous) => {
        const next = { ...previous };
        if (value === (current ?? '')) delete next[key];
        else next[key] = value;
        return next;
      });
    },
    clear: () => setChanges({}),
  };
}

/** Una voce: viva (si modifica) o non disponibile (spenta, con il motivo). */
export function SettingField({ row, label, doc, draft, locked = false }: { row: SettingRow; label: string; doc: EngineSettings; draft: Draft; locked?: boolean }) {
  const t = useT();
  const item = doc.items[row.key];
  const live = item?.state === 'live';
  const current = doc.values[row.key];
  const changed = draft.changes[row.key];
  const value = changed ?? current ?? '';
  const id = `ens-${row.key}`;
  const change = (next: string) => {
    draft.set(row.key, next, current);
    // in JTDX le due caselle di ALL.TXT si escludono: accenderne una spegne l'altra
    const other = row.key === 'WriteDecodedALLTXT' ? 'WriteDecodedDebugALLTXT' : row.key === 'WriteDecodedDebugALLTXT' ? 'WriteDecodedALLTXT' : null;
    if (other && next === 'true') draft.set(other, 'false', doc.values[other]);
  };
  let control;
  if (row.kind === 'bool') {
    control = <input id={id} aria-label={label} type="checkbox" disabled={!live || locked} checked={live && value === 'true'} onChange={(e) => change(e.target.checked ? 'true' : 'false')} />;
  } else if (row.kind === 'num') {
    control = <input id={id} aria-label={label} type="number" min={row.min ?? 1} max={row.max ?? 65535} disabled={!live || locked} className="set-number" value={live ? value : ''} onChange={(e) => change(e.target.value)} />;
  } else if (row.kind === 'reply') {
    control = (
      <select id={id} aria-label={label} disabled={!live || locked} value={live ? value : ''} onChange={(e) => change(e.target.value)}>
        {!live ? <option value="" /> : null}
        {['1', '2', '3'].map((o) => (
          <option key={o} value={o}>
            {t(`jx.reply.${o}` as Key)}
          </option>
        ))}
      </select>
    );
  } else {
    // le credenziali di eQSL non si salvano: il campo resta vuoto e spento
    control = <input id={id} aria-label={label} type={row.kind === 'secret' ? 'password' : 'text'} className="jx-text" disabled={!live || locked} maxLength={200} value={live ? value : ''} placeholder={row.kind === 'secret' && doc.secrets?.[row.key] ? t('ens.pw.saved') : undefined} autoComplete="off" onChange={(e) => change(e.target.value)} />;
  }
  const note = live ? (item?.apply === 'restart' ? t('ens.restart') : '') : t(`ens.r.${item?.reason ?? 'own_decoder'}` as Key);
  const liveNote = live && item?.note ? t(`ens.n.${item.note}` as Key) : '';
  return (
    <label className={`jx-row${changed !== undefined ? ' jx-row--changed' : ''}${live ? '' : ' jx-row--off'}`} htmlFor={id} title={live ? undefined : note}>
      <span className="jx-name">
        {label}
        {note ? <small className="ens-note">{live ? ` (${note})` : ` — ${t('ens.off')}`}</small> : null}
      </span>
      {control}
      {!live ? <span className="ens-reason muted">{note}</span> : null}
      {liveNote ? <span className="ens-reason muted">{liveNote}</span> : null}
    </label>
  );
}

/** Barra in fondo: quante modifiche aspettano, con «Applica» (subito, niente riavvii) e «Annulla». */
export function ApplyBar({ draft, doc }: { draft: Draft & { clear(): void }; doc: EngineSettings }) {
  const t = useT();
  const client = useQueryClient();
  const [note, setNote] = useState('');
  const changes = draft.changes;
  const apply = useMutation({
    mutationFn: () => putEngineSettings(changes),
    onSuccess: (next) => {
      const slow = Object.keys(changes).some((k) => doc.items[k]?.apply === 'restart');
      client.setQueryData(['engine-settings'], next);
      draft.clear();
      setNote(slow ? t('ens.appliedRestart') : t('ens.applied'));
    },
    onError: (e) => setNote(errorText(e, t)),
  });
  const count = Object.keys(changes).length;
  if (count === 0 && !note) return null;
  return (
    <div className="jx-bar" role="status">
      {count > 0 ? <strong>{t('ens.pending', { n: count })}</strong> : null}
      {count > 0 ? (
        <>
          <button type="button" className="glass glass--on" disabled={apply.isPending} onClick={() => apply.mutate()}>
            {t('ens.apply')}
          </button>
          <button type="button" className="glass" disabled={apply.isPending} onClick={() => { draft.clear(); setNote(''); }}>
            {t('ens.discard')}
          </button>
        </>
      ) : null}
      {note ? <span className="live-note">{note}</span> : null}
    </div>
  );
}

const STATUS_OF: Record<string, string> = { PSKReporter: 'psk', EQSLSend: 'eqsl', EnableTCPConnection: 'tcp', AllowSpotsDXSummit: 'dxsummit' };

/** L'esito dell'ultimo invio di un servizio (ok o errore, e quando). */
function SendStatus({ doc, kind }: { doc: EngineSettings; kind: string }) {
  const t = useT();
  const st = doc.status?.[kind];
  const text = !st || st.ok === null ? t('ens.st.never') : `${st.ok ? t('ens.st.ok') : t('ens.st.err')}: ${st.message} (${new Date(st.at_ms).toLocaleTimeString()})`;
  return (
    <p className={`ens-reason ${st?.ok === false ? 'live-note' : 'muted'}`} role="status" data-testid={`status-${kind}`}>
      {t('ens.st.title')}: {text}
    </p>
  );
}

/** La scheda Segnalazioni come in JTDX, nel sistema di layout delle schede: cinque riquadri a tutta larghezza, voci in celle, aiuti in «Note», voci spente in un riquadro chiuso. */
export function NativeReports() {
  const t = useT();
  const settings = useSettings();
  const draft = useDraft();
  if (settings.isError) return <p className="muted">{t('ens.unavailable')}</p>;
  if (!settings.data) return <p className="muted">{t('set.loading')}</p>;
  const doc = settings.data;
  const off = REPORT_GROUPS.flatMap((g) => g.rows).filter((r) => doc.items[r.key]?.state !== 'live');
  return (
    <section aria-label={t('ens.title')}>
      <Page>
        {REPORT_GROUPS.map((group) => {
          const rows = group.rows.filter((r) => doc.items[r.key]?.state === 'live');
          const kinds = [...new Set(rows.map((r) => STATUS_OF[r.key]).filter((k): k is string => Boolean(k)))];
          return (
            <Group key={group.title} title={t(group.title)} span={group.title === 'ens.g.net' || group.title === 'ens.g.udp' ? 2 : 1}>
              <Cells min={15}>
                {rows.map((row) => (
                  <SettingField key={row.key} row={row} label={t(`ens.k.${row.key}` as Key)} doc={doc} draft={draft} />
                ))}
              </Cells>
              {kinds.map((k) => (
                <SendStatus key={k} doc={doc} kind={k} />
              ))}
            </Group>
          );
        })}
        <Notes span={2}>
          <Help>{t('ens.rep.help')}</Help>
          {doc.items.AllowSpotsDXSummit?.state === 'live' ? <Help>{t('ens.dxsummit.help')}</Help> : null}
          <Help>{t('ens.note', { file: doc.file })}</Help>
        </Notes>
        <Unavailable count={off.length} span={2}>
          <Cells min={15}>
            {off.map((row) => (
              <SettingField key={row.key} row={row} label={t(`ens.k.${row.key}` as Key)} doc={doc} draft={draft} />
            ))}
          </Cells>
        </Unavailable>
      </Page>
      <ApplyBar draft={draft} doc={doc} />
    </section>
  );
}

/** Le voci di un gruppo di schede (Generali, Radio, Audio, Sequenza, Notifiche, Avanzate) con il motore proprio: quelle vive si modificano, le altre sono elencate con il motivo. */
export function NativeValues({ group, title }: { group: EngineGroup; title?: string }) {
  const t = useT();
  const settings = useSettings();
  const draft = useDraft();
  if (settings.isError) return <p className="muted">{t('ens.unavailable')}</p>;
  if (!settings.data) return <p className="muted">{t('set.loading')}</p>;
  const doc = settings.data;
  const items = ENGINE_GROUPS[group];
  const live = items.filter((i) => doc.items[i.key]?.state === 'live');
  const off = items.filter((i) => doc.items[i.key]?.state !== 'live');
  return (
    <section aria-label={title ?? t('ens.title')}>
      <h3 className="set-sub">{title ?? t('ens.title')}</h3>
      {live.length > 0 ? (
        <>
          <div className="jx-list">
            {live.map((i) => (
              <SettingField key={i.key} row={{ key: i.key, kind: i.kind === 'bool' ? 'bool' : i.kind === 'num' ? 'num' : i.kind === 'text' ? 'text' : 'reply' }} label={t(`jx.${i.key}` as Key)} doc={doc} draft={draft} />
            ))}
          </div>
          <Help>{t('ens.note', { file: doc.file })}</Help>
          <ApplyBar draft={draft} doc={doc} />
        </>
      ) : null}
      {off.length > 0 ? (
        <details className="ens-hidden">
          <summary>{t('ens.hidden', { n: off.length })}</summary>
          <ul className="ens-hidden-list">
            {off.map((i) => (
              <li key={i.key}>
                <strong>{t(`jx.${i.key}` as Key)}</strong>
                <span className="muted"> — {t(`ens.r.${doc.items[i.key]?.reason ?? 'own_decoder'}` as Key)}</span>
              </li>
            ))}
          </ul>
        </details>
      ) : null}
    </section>
  );
}
