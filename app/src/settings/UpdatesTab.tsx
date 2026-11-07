import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { openUpdatePoster, UpdatePoster } from '../components/UpdatePoster';
import { ApiError, checkUpdate, getUpdate, installUpdate, putUpdate, type UpdateConfigInput } from '../api/client';
import { useT } from '../i18n';
import { Check, Help } from './common';
import { Group, Message, Notes, Page } from './layout';
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
  const [installing, setInstalling] = useState(false);
  const [installFailure, setInstallFailure] = useState('');
  const install = async () => {
    setInstalling(true);
    setInstallFailure('');
    try {
      await installUpdate();
    } catch (error) {
      setInstalling(false);
      setInstallFailure(t('upd.installFailed', { message: error instanceof ApiError ? error.code : '—' }));
    }
  };

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
      // si apre sempre la locandina: con la versione nuova, oppure con «Aurora e' aggiornata»
      if (!result.error) openUpdatePoster();
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
    <Page className="upd-page">
      {/* Impostazioni in una finestra a parte (sempre davanti): la locandina deve comparire qui, non dietro nella finestra principale */}
      {location.hash.startsWith('#/finestra/') ? <UpdatePoster onlyWhenAsked /> : null}
      {/* Versione: tre numeri grandi in fila, lo stato in una riga sola, i pulsanti sotto */}
      <Group title={t('lay.status')}>
        <div className="upd-stats">
          <div className="upd-stat">
            <span className="upd-stat-label">{t('upd.current')}</span>
            <span className="upd-stat-value data">{status.current}</span>
          </div>
          <div className="upd-stat">
            <span className="upd-stat-label">{t('upd.latest')}</span>
            <span className="upd-stat-value data">
              {status.latest ? (
                status.available ? (
                  // versione nuova: ben visibile, con la stella e «NEW»
                  <span className="upd-new">
                    <span className="upd-new-star" aria-hidden="true">★</span>
                    {status.latest.version}
                    <span className="upd-new-tag">NEW</span>
                  </span>
                ) : (
                  status.latest.version
                )
              ) : (
                '—'
              )}
            </span>
          </div>
          <div className="upd-stat">
            <span className="upd-stat-label">{t('upd.lastCheck')}</span>
            <span className="upd-stat-value data">{status.checked_at_ms > 0 ? new Date(status.checked_at_ms).toLocaleString() : t('upd.never')}</span>
          </div>
        </div>
        <div className="upd-status-row">
          <div className="upd-status-text">
            {status.available ? (
              <span className="upd-status upd-status--new" role="status">
                {t('upd.available', { version: status.available.version })}{' '}
                {status.available.url ? (
                  <a href={status.available.url} target="_blank" rel="noreferrer noopener">
                    {t('upd.open')}
                  </a>
                ) : null}
              </span>
            ) : status.latest ? (
              <span className="upd-status upd-status--ok">✓ {t('upd.uptodate')}</span>
            ) : null}
          </div>
          <div className="upd-buttons">
            {status.available && status.can_install ? (
              <button type="button" className="glass glass--on" disabled={installing} onClick={() => void install()}>
                {t('upd.install')}
              </button>
            ) : null}
            <button type="button" className="glass" disabled={check.isPending || draft !== null} onClick={() => check.mutate()}>
              {t('upd.check')}
            </button>
          </div>
        </div>
        {status.error ? <Message kind="alert">{t('upd.offline', { message: status.error })}</Message> : null}
        {installing ? <Message kind="muted">{t('upd.installing')}</Message> : null}
        {installFailure ? <Message kind="alert">{installFailure}</Message> : null}
      </Group>

      {/* Controllo automatico: interruttore e ogni quante ore, sulla stessa riga */}
      <Group title={t('lay.settings')}>
        <div className="upd-auto">
          <Check label={t('upd.enabled')} hint={t('tip.upd.enabled')} checked={shown.enabled} onChange={(v) => set({ enabled: v })} />
          {/* repository, versioni di prova e token restano quelli di fabbrica: servono solo a chi sviluppa, non a chi usa Aurora */}
          <label className="upd-hours" title={t('tip.upd.interval')}>
            <span>{t('upd.interval')}</span>
            <input type="number" className="jx-text upd-hours-input" min={1} max={168} value={shown.hours} disabled={!shown.enabled} onChange={(e) => set({ hours: e.target.value })} />
          </label>
        </div>
        {!parsed ? <Message kind="alert">{t('upd.invalid')}</Message> : null}
        {note ? <Message>{note}</Message> : null}
      </Group>


      <Notes>
        <Help>{t('upd.intro')}</Help>
        <Help>{t('tip.upd.token')}</Help>
      </Notes>
    </Page>
  );
}
