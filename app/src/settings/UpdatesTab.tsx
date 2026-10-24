import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { checkUpdate, getUpdate, putUpdate, type UpdateConfigInput } from '../api/client';
import { useT } from '../i18n';
import { Check, Help, Row } from './common';
import { useAutosave } from './useAutosave';

interface Draft {
  enabled: boolean;
  repo: string;
  hours: string;
  prereleases: boolean;
  /** Il token nuovo scritto qui; vuoto = non lo si cambia. */
  token: string;
  removeToken: boolean;
}

/** Il testo della scheda in un'impostazione valida per il core, o `null` se i valori non vanno. */
export function draftToUpdate(d: Draft): UpdateConfigInput | null {
  const repo = d.repo.trim();
  const hours = Number(d.hours);
  if (!/^[A-Za-z0-9_.-]{1,100}\/[A-Za-z0-9_.-]{1,100}$/.test(repo)) return null;
  if (!Number.isInteger(hours) || hours < 1 || hours > 168) return null;
  const token = d.token.trim();
  if (token.length > 255 || /\s/.test(token)) return null;
  const input: UpdateConfigInput = { enabled: d.enabled, repo, interval_hours: hours, prereleases: d.prereleases };
  if (d.removeToken) input.token = '';
  else if (token !== '') input.token = token;
  return input;
}

/** Scheda Aggiornamenti: dove e quando cercare le nuove versioni di Aurora. */
export function UpdatesTab() {
  const t = useT();
  const client = useQueryClient();
  const update = useQuery({ queryKey: ['update'], queryFn: getUpdate, retry: false });
  const [draft, setDraft] = useState<Draft | null>(null);
  const [note, setNote] = useState('');

  const status = update.data;
  const shown: Draft | null = draft ?? (status ? { enabled: status.enabled, repo: status.repo, hours: String(status.interval_hours), prereleases: status.prereleases, token: '', removeToken: false } : null);
  const parsed = shown ? draftToUpdate(shown) : null;

  const save = useMutation({
    mutationFn: (input: UpdateConfigInput) => putUpdate(input),
    onSuccess: () => {
      setDraft(null);
      setNote(t('upd.saved'));
      void client.invalidateQueries({ queryKey: ['update'] });
    },
    onError: () => setNote(t('upd.invalid')),
  });
  const check = useMutation({
    mutationFn: () => checkUpdate(),
    onSuccess: (result) => {
      setNote(result.error ? t('upd.offline', { message: result.error }) : '');
      void client.invalidateQueries({ queryKey: ['update'] });
    },
    onError: () => setNote(t('upd.offline', { message: '—' })),
  });

  useAutosave(
    parsed,
    draft !== null,
    (input) => save.mutate(input),
    (input) => void putUpdate(input).catch(() => undefined),
  );

  if (update.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!status || !shown) return <p className="muted">{t('set.loading')}</p>;
  const set = (change: Partial<Draft>) => setDraft({ ...shown, ...change });

  return (
    <>
      <Help>{t('upd.intro')}</Help>
      <Row name={t('upd.current')}>{status.current}</Row>
      <Row name={t('upd.latest')}>{status.latest ? status.latest.version : '—'}</Row>
      <Row name={t('upd.lastCheck')}>{status.checked_at_ms > 0 ? new Date(status.checked_at_ms).toLocaleString() : t('upd.never')}</Row>
      {status.available ? (
        <p className="live-note" role="status">
          {t('upd.available', { version: status.available.version })}{' '}
          {status.available.url ? (
            <a href={status.available.url} target="_blank" rel="noreferrer noopener">
              {t('upd.open')}
            </a>
          ) : null}
        </p>
      ) : status.latest ? (
        <p className="muted">{t('upd.uptodate')}</p>
      ) : null}
      {status.error ? <p className="notice">{t('upd.offline', { message: status.error })}</p> : null}
      <div className="cmd-chips">
        <button type="button" className="glass" disabled={check.isPending || draft !== null} onClick={() => check.mutate()}>
          {t('upd.check')}
        </button>
      </div>

      <Check label={t('upd.enabled')} checked={shown.enabled} onChange={(v) => set({ enabled: v })} />
      <label className="set-check" title={t('tip.upd.repo')}>
        <span>{t('upd.repo')}</span>
        <input type="text" className="jx-text" value={shown.repo} spellCheck={false} onChange={(e) => set({ repo: e.target.value })} />
      </label>
      <label className="set-check">
        <span>{t('upd.interval')}</span>
        <input type="number" className="set-number" min={1} max={168} value={shown.hours} onChange={(e) => set({ hours: e.target.value })} />
      </label>
      <Check label={t('upd.prereleases')} hint={t('tip.upd.prereleases')} checked={shown.prereleases} onChange={(v) => set({ prereleases: v })} />
      <label className="set-check" title={t('tip.upd.token')}>
        <span>{t('upd.token')}</span>
        <input
          type="password"
          className="jx-text"
          value={shown.token}
          autoComplete="off"
          spellCheck={false}
          placeholder={status.has_token ? t('upd.tokenPlaceholder') : ''}
          onChange={(e) => set({ token: e.target.value, removeToken: false })}
        />
      </label>
      {status.has_token ? (
        <p className="muted">
          {t('upd.tokenSaved')}{' '}
          <button type="button" className="glass" onClick={() => set({ token: '', removeToken: true })}>
            {t('upd.tokenRemove')}
          </button>
        </p>
      ) : null}
      <Help>{t('tip.upd.token')}</Help>
      {!parsed ? <p className="notice">{t('upd.invalid')}</p> : <Help>{t('upd.autosave')}</Help>}
      {note ? <p className="live-note" role="status">{note}</p> : null}
    </>
  );
}
