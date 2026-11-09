import { useEffect, useState } from 'react';
import { ApiError, chooseLog4omDb, getLog4omDb, setLog4omDb, type Log4omDb } from '../api/client';
import { useT } from '../i18n';

/**
 * Riquadro «Database di Log4OM»: il file SQLite di Log4OM2 da cui Aurora legge i QSO gia' fatti (etichette «gia' lavorato / nuovo
 * DXCC»). Si sceglie con la finestra «Apri file» (programma Windows, dal PC stesso) o scrivendo il percorso; «Togli» non lo usa piu'.
 * Il core controlla il file, lo ricorda e ricarica subito i QSO, senza riavviare.
 */
export function Log4omDbSection() {
  const t = useT();
  const [db, setDb] = useState<Log4omDb | null>(null);
  const [typed, setTyped] = useState('');
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState('');
  const [failure, setFailure] = useState('');

  useEffect(() => {
    getLog4omDb()
      .then(setDb)
      .catch(() => setDb(null));
  }, []);

  const run = async (action: () => Promise<Log4omDb | { cancelled: true }>) => {
    setBusy(true);
    setDone('');
    setFailure('');
    try {
      const r = await action();
      if ('cancelled' in r) return;
      setDb((old) => ({ ...r, dialog: old?.dialog }));
      setTyped('');
      setDone(r.path ? t('l4.saved', { count: r.count ?? 0 }) : t('l4.removed'));
    } catch (error) {
      if (error instanceof ApiError && error.code === 'log4om_db_invalid') setFailure(t('l4.invalid', { message: error.detail ?? '' }));
      else if (error instanceof ApiError && error.code === 'log4om_db_locked') setFailure(t('l4.locked'));
      else if (error instanceof ApiError && error.code === 'local_only') setFailure(t('l4.localOnly'));
      else setFailure(t('l4.failed', { message: error instanceof ApiError ? error.code : '—' }));
    } finally {
      setBusy(false);
    }
  };

  if (!db) return null;
  const source = db.source === 'auto' ? t('l4.srcAuto') : db.source === 'chosen' ? t('l4.srcChosen') : db.source === 'env' ? t('l4.srcEnv') : '';
  return (
    <section className="about-fact about-support">
      <h3>{t('l4.title')}</h3>
      <p>{t('l4.intro')}</p>
      <p>
        <code>{db.path ?? t('l4.none')}</code>
        {source && <span className="muted"> — {source}</span>}
        {db.path && !db.exists && <span className="muted"> — {t('l4.missing')}</span>}
      </p>
      {db.locked ? (
        <p className="muted">{t('l4.locked')}</p>
      ) : (
        <div className="about-support-actions">
          {db.dialog && (
            <button type="button" className="glass about-support-main" disabled={busy} onClick={() => void run(chooseLog4omDb)}>
              {t('l4.choose')}
            </button>
          )}
          <input type="text" className="glass" placeholder={t('l4.pathPlaceholder')} aria-label={t('l4.pathPlaceholder')} value={typed} disabled={busy} onChange={(e) => setTyped(e.target.value)} />
          <button type="button" className="glass" disabled={busy || typed.trim() === ''} onClick={() => void run(() => setLog4omDb(typed.trim()))}>
            {t('l4.use')}
          </button>
          <button type="button" className="glass" disabled={busy || !db.path} onClick={() => void run(() => setLog4omDb(null))}>
            {t('l4.remove')}
          </button>
        </div>
      )}
      {done && <p role="status">{done}</p>}
      {failure && <p role="alert">{failure}</p>}
    </section>
  );
}
