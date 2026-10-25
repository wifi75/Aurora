import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { ApiError, getEngineSettings, getStationInfo, putEngineSettings } from '../api/client';
import type { EngineSettings } from '../api/schemas';
import { useT, type Key, type T } from '../i18n';
import { ENGINE_GROUPS, type EngineGroup } from '../lib/engineGroups';
import { REPORT_GROUPS, type SettingRow } from '../lib/engineSettingsLayout';
import { Help } from './common';

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
  const change = (next: string) => draft.set(row.key, next, current);
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
    control = <input id={id} aria-label={label} type={row.kind === 'secret' ? 'password' : 'text'} className="jx-text" disabled={!live || locked} maxLength={200} value={live ? value : ''} autoComplete="off" onChange={(e) => change(e.target.value)} />;
  }
  const note = live ? (item?.apply === 'restart' ? t('ens.restart') : '') : t(`ens.r.${item?.reason ?? 'own_decoder'}` as Key);
  return (
    <label className={`jx-row${changed !== undefined ? ' jx-row--changed' : ''}${live ? '' : ' jx-row--off'}`} htmlFor={id} title={live ? undefined : note}>
      <span className="jx-name">
        {label}
        {note ? <small className="ens-note">{live ? ` (${note})` : ` — ${t('ens.off')}`}</small> : null}
      </span>
      {control}
      {!live ? <span className="ens-reason muted">{note}</span> : null}
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

/** La scheda Segnalazioni come in JTDX: cinque gruppi con le stesse voci; le voci che Aurora non ha ancora si vedono spente, con il motivo. */
export function NativeReports() {
  const t = useT();
  const settings = useSettings();
  const draft = useDraft();
  if (settings.isError) return <p className="muted">{t('ens.unavailable')}</p>;
  if (!settings.data) return <p className="muted">{t('set.loading')}</p>;
  const doc = settings.data;
  return (
    <section aria-label={t('ens.title')}>
      <Help>{t('ens.rep.help')}</Help>
      {REPORT_GROUPS.map((group) => (
        <fieldset key={group.title} className="ens-group">
          <legend className="set-sub">{t(group.title)}</legend>
          <div className="jx-list">
            {group.rows.map((row) => (
              <SettingField key={row.key} row={row} label={t(`ens.k.${row.key}` as Key)} doc={doc} draft={draft} />
            ))}
          </div>
        </fieldset>
      ))}
      <Help>{t('ens.note', { file: doc.file })}</Help>
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
