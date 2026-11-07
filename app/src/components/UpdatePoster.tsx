import '../styles/first-run.css';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { useStation } from '../state/station';
import { ApiError, getUpdate, installUpdate } from '../api/client';
import { useLang, useT } from '../i18n';
import { pick } from '../i18n/core';
import { NOVITA } from '../lib/novita';
import { browserStorage } from '../lib/theme';

const SEEN_KEY = 'aurora-update-poster-seen';
const OPEN_EVENT = 'aurora-update-poster-open';

/** «Controlla aggiornamenti» premuto a mano: la locandina si riapre sempre (anche se rimandata con «Piu' tardi», o se si e' gia' aggiornati). */
export function openUpdatePoster(): void {
  window.dispatchEvent(new Event(OPEN_EVENT));
}

function seenVersion(): string {
  try {
    return browserStorage()?.getItem(SEEN_KEY) ?? '';
  } catch {
    return '';
  }
}

/** Le note della release in punti brevi: una riga per frase (al massimo sei), senza i segni del Markdown. */
export function noteLines(notes: string): string[] {
  return notes
    .replace(/[#*_`>]/g, '')
    .split(/\n+|(?<=[.;])\s+(?=[A-ZÀ-Ý«])/)
    .map((l) => l.replace(/^[-•\s]+/, '').trim())
    .filter((l) => l.length > 2)
    .slice(0, 6);
}

/**
 * La locandina della versione nuova: all'apertura, se sul repository c'e' una versione piu' nuova, compare al centro con le novita' e il
 * pulsante per installarla. «Piu' tardi» la nasconde fino alla versione dopo (l'icona in testata resta).
 */
export function UpdatePoster() {
  const t = useT();
  const lang = useLang((s) => s.lang);
  const update = useQuery({ queryKey: ['update'], queryFn: getUpdate, retry: false, refetchInterval: (q) => ((q.state.data?.checked_at_ms ?? 0) === 0 ? 5_000 : 10 * 60_000) });
  const [seen, setSeen] = useState(seenVersion);
  const [installing, setInstalling] = useState(false);
  const [failure, setFailure] = useState('');
  // il core avvisa appena trova la versione nuova: si rileggono i dettagli (note, data, installazione)
  const notice = useStation((st) => st.updateNotice);
  const refetch = update.refetch;
  useEffect(() => {
    if (notice) void refetch();
  }, [notice, refetch]);
  // aperta a mano: si mostra anche se rimandata o se non c'e' niente di nuovo (allora dice che Aurora e' aggiornata)
  const [forced, setForced] = useState(false);
  useEffect(() => {
    const open = () => {
      setForced(true);
      void refetch();
    };
    window.addEventListener(OPEN_EVENT, open);
    return () => window.removeEventListener(OPEN_EVENT, open);
  }, [refetch]);
  const release = update.data?.available ?? (notice ? { version: notice.version, tag: notice.tag, name: '', notes: '', url: notice.url, published_at: '', prerelease: false } : null);
  if (!release && forced) return <UpToDate current={update.data?.current ?? ''} onClose={() => setForced(false)} />;
  if (!release || (seen === release.version && !forced)) return null;

  const later = () => {
    try {
      browserStorage()?.setItem(SEEN_KEY, release.version);
    } catch {
      // senza archivio la locandina torna alla prossima apertura
    }
    setSeen(release.version);
    setForced(false);
  };
  const install = async () => {
    setInstalling(true);
    setFailure('');
    try {
      await installUpdate();
    } catch (error) {
      setInstalling(false);
      setFailure(t('upd.installFailed', { message: error instanceof ApiError ? error.code : '—' }));
    }
  };
  const when = release.published_at ? new Date(release.published_at).toLocaleString(lang, { dateStyle: 'short', timeStyle: 'short' }) : '';
  const lines = noteLines(release.notes || release.name || '');

  return (
    <div className="first-run upd-poster" role="dialog" aria-modal="true" aria-labelledby="upd-poster-title">
      <div className="first-run__card upd-poster__card">
        <div className="upd-poster__art">
          <svg viewBox="0 0 560 150" preserveAspectRatio="none" aria-hidden="true">
            <g fill="none" stroke="#fff" strokeOpacity=".35" strokeWidth="1.5">
              <path d="M0 110 Q140 40 280 90 T560 60" />
              <path d="M0 130 Q140 70 280 110 T560 90" />
              <path d="M0 90 Q140 20 280 70 T560 40" />
            </g>
          </svg>
          <div className="upd-poster__head">
            <span id="upd-poster-title" className="upd-poster__name">Aurora {release.version}</span>
            <span>{t('upd.poster.available')}</span>
          </div>
        </div>
        <div className="upd-poster__body">
          {lines.length > 0 ? (
            <>
              <span className="upd-poster__pill">{t('upd.poster.news')}</span>
              <ul className="upd-poster__list">
                {lines.map((l) => (
                  <li key={l}>{l}</li>
                ))}
              </ul>
            </>
          ) : null}
          <p className="upd-poster__ver">{t('upd.poster.current', { current: update.data?.current ?? '…', when })}</p>
          {failure ? (
            <p className="first-run__error" role="alert">
              {failure}
            </p>
          ) : null}
          <div className="upd-poster__actions">
            {release.url ? (
              <a className="first-run__later" href={release.url} target="_blank" rel="noreferrer noopener">
                {t('upd.open')}
              </a>
            ) : null}
            <button type="button" className="first-run__later" onClick={later} disabled={installing}>
              {t('upd.later')}
            </button>
            {update.data?.can_install ? (
              <button type="button" className="first-run__next" onClick={() => void install()} disabled={installing}>
                {installing ? t('upd.installing') : t('upd.poster.install')}
              </button>
            ) : null}
          </div>
        </div>
      </div>
    </div>
  );
}

/** La locandina quando si e' gia' all'ultima versione: lo dice, con le novita' della versione installata. */
function UpToDate({ current, onClose }: { current: string; onClose(): void }) {
  const t = useT();
  const lang = useLang((s) => s.lang);
  const mine = NOVITA.find((n) => n.version === current) ?? NOVITA[0];
  const when = mine ? new Date(mine.released).toLocaleString(lang, { dateStyle: 'short', timeStyle: 'short' }) : '';
  return (
    <div className="first-run upd-poster" role="dialog" aria-modal="true" aria-labelledby="upd-poster-title" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="first-run__card upd-poster__card">
        <div className="upd-poster__art">
          <svg viewBox="0 0 560 150" preserveAspectRatio="none" aria-hidden="true">
            <g fill="none" stroke="#fff" strokeOpacity=".35" strokeWidth="1.5">
              <path d="M0 110 Q140 40 280 90 T560 60" />
              <path d="M0 130 Q140 70 280 110 T560 90" />
              <path d="M0 90 Q140 20 280 70 T560 40" />
            </g>
          </svg>
          <div className="upd-poster__head">
            <span id="upd-poster-title" className="upd-poster__name">Aurora {current || mine?.version}</span>
            <span>{t('upd.poster.upToDate')}</span>
          </div>
        </div>
        <div className="upd-poster__body">
          {mine ? (
            <>
              <span className="upd-poster__pill">{t('upd.poster.news')}</span>
              <ul className="upd-poster__list">
                <li>
                  <b>{pick(mine.title, lang)}</b>
                </li>
                <li>{pick(mine.text, lang)}</li>
              </ul>
              <p className="upd-poster__ver">{t('upd.poster.installed', { when })}</p>
            </>
          ) : null}
          <div className="upd-poster__actions">
            <a className="first-run__later" href="#/novita" onClick={onClose}>
              {t('upd.poster.allNews')}
            </a>
            <button type="button" className="first-run__next" onClick={onClose}>
              {t('upd.poster.ok')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
