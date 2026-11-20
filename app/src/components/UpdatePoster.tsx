import '../styles/first-run.css';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import { useStation } from '../state/station';
import { ApiError, getUpdate, installUpdate } from '../api/client';
import { useLang, useT } from '../i18n';
import { localeOf, pick } from '../i18n/core';
import { NOVITA } from '../lib/novita';
import { browserStorage } from '../lib/theme';
import { InstallProgress } from './InstallProgress';

const SEEN_KEY = 'aurora-update-poster-seen';
const OPEN_EVENT = 'aurora-update-poster-open';
const DAY_KEY = 'aurora-update-poster-day';
/** All'avvio di Aurora la locandina si apre da sola, ma solo nei primi due minuti (poi un QSO o un contest non si disturbano). */
const STARTUP_MS = 120_000;

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

/** «Piu' tardi» vale fino a domani: all'avvio dopo, se Aurora non e' ancora aggiornata, la locandina torna. */
function snoozedToday(): boolean {
  try {
    return browserStorage()?.getItem(DAY_KEY) === today();
  } catch {
    return false;
  }
}

/** «Piu' tardi» premuto: la guida del primo avvio, che aspettava la locandina, puo' partire. */
export const LATER_EVENT = 'aurora-update-later';

/** La versione rimandata con «Piu' tardi» (vuoto se nessuna). */
export function readSeenVersion(): string {
  try {
    return browserStorage()?.getItem(SEEN_KEY) ?? '';
  } catch {
    return '';
  }
}

/** La guida del primo avvio non e' ancora stata fatta (stessa chiave di FirstRun). */
function firstRunPending(): boolean {
  try {
    return browserStorage()?.getItem('aurora-first-run-done') !== '1';
  } catch {
    return false;
  }
}

/** «Controlla aggiornamenti» premuto a mano: la locandina si riapre sempre (anche se rimandata con «Piu' tardi», o se si e' gia' aggiornati). */
export function openUpdatePoster(): void {
  window.dispatchEvent(new Event(OPEN_EVENT));
}


/** Le note della release in punti brevi: una riga per frase (al massimo sei), senza i segni del Markdown. */
const NOTE_LANG_NAMES: Record<string, string> = { it: 'Italiano', de: 'Deutsch', fr: 'Français', es: 'Español', pt: 'Português', nl: 'Nederlands' };

/** La parte delle note nella lingua `lang`: il blocco `<details>` della lingua, altrimenti l'inglese in cima (senza «Download:»). */
export function notesForLang(notes: string, lang: string): string {
  const name = NOTE_LANG_NAMES[lang];
  if (name) {
    for (const m of notes.matchAll(/<details>([\s\S]*?)(?:<\/details>|$)/g)) {
      const block = m[1] ?? '';
      const end = block.indexOf('</summary>');
      if (end >= 0 && block.slice(0, end).includes(name)) return block.slice(end + '</summary>'.length).trim();
    }
  }
  return (notes.split('<details>')[0] ?? notes)
    .split('\n')
    .filter((l) => !l.trimStart().startsWith('Download:'))
    .join('\n')
    .trim();
}

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
/**
 * `onlyWhenAsked`: la locandina della finestra delle Impostazioni (che sta sempre davanti alla principale): compare solo quando si preme
 * «Controlla ora», mai da sola, cosi' non si doppia con quella della finestra principale.
 */
// `onlyWhenAsked` resta per chi lo passa: ora la locandina si apre sempre e solo a richiesta
export function UpdatePoster(_props: { onlyWhenAsked?: boolean } = {}) {
  const t = useT();
  const lang = useLang((s) => s.lang);
  const update = useQuery({ queryKey: ['update'], queryFn: getUpdate, retry: false, refetchInterval: (q) => ((q.state.data?.checked_at_ms ?? 0) === 0 ? 5_000 : 10 * 60_000) });
  const [seen, setSeen] = useState('');
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
  // appena Aurora si apre, se c'e' una versione nuova (e non e' stata rimandata oggi) la locandina compare al centro, una volta sola
  const born = useRef(Date.now());
  const releaseVersion = release?.version;
  const phase = update.data?.stage?.phase;
  const quiet = update.data?.quiet;
  useEffect(() => {
    if (!releaseVersion || Date.now() - born.current > STARTUP_MS || quiet || phase === 'installing' || snoozedToday()) return;
    // all'avvio, con una versione nuova, si apre direttamente la locandina con tutte le novita' (come quando la si chiede a mano)
    setForced(true);
  }, [releaseVersion, quiet, phase]);
  if (!release && forced) return <UpToDate current={update.data?.current ?? ''} onClose={() => setForced(false)} />;
  // la locandina non si apre mai da sola (comparire durante un QSO o un contest e' un disturbo): avvisano la cartolina del tray e la
  // barretta in testata, e la locandina si apre solo quando l'operatore la chiede (clic sulla barretta o dal menu)
  if (!forced) return null;
  // aggiornamento chiesto dal tray: niente locandina, fa tutto la cartolina del tray
  if (!forced && update.data?.quiet) return null;
  // con l'aggiornamento scaricato in anticipo la barretta in testata (e la cartolina del tray) mostrano tutto: la locandina si apre solo
  // quando la si chiede (clic sulla barretta). Prima compariva da sola mentre si aggiornava dal tray e si chiudeva dopo pochi secondi
  if (!forced && update.data?.stage?.phase === 'installing') return null;
  if (!release || (seen === release.version && !forced)) return null;

  const later = () => {
    try {
      browserStorage()?.setItem(SEEN_KEY, release.version);
      browserStorage()?.setItem(DAY_KEY, today());
    } catch {
      // senza archivio la locandina torna alla prossima apertura
    }
    setSeen(release.version);
    setForced(false);
    window.dispatchEvent(new Event(LATER_EVENT));
  };
  const install = async () => {
    setInstalling(true);
    setFailure('');
    try {
      await installUpdate();
    } catch (error) {
      setInstalling(false);
      setFailure(t('upd.installFailed', { message: error instanceof ApiError ? (error.detail ?? error.code) : '—' }));
    }
  };
  // l'installazione parte SOLO quando l'operatore preme «Installa»: mai da sola (in contest o durante un DX chiudere Aurora sarebbe un danno)
  const when = release.published_at ? new Date(release.published_at).toLocaleString(localeOf(lang), { dateStyle: 'short', timeStyle: 'short' }) : '';
  const lines = noteLines(notesForLang(release.notes || release.name || '', lang));

  return (
    <div className="first-run upd-poster" role="dialog" aria-modal="true" aria-labelledby="upd-poster-title">
      <div className="first-run__card upd-poster__card">
        <div className="upd-poster__art">
          <span className="upd-poster__ribbon" aria-hidden="true">{t('upd.poster.ribbon')}</span>
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
          {/* primo avvio con un installatore vecchio: prima l'aggiornamento, poi la guida (scelta del proprietario, 2026-11-10) */}
          {firstRunPending() && !forced ? <p className="upd-poster__first">{t('upd.poster.firstRun')}</p> : null}
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
          <InstallProgress active={installing} />
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
                {installing ? t('upd.installShort') : t('upd.poster.install')}
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
  const when = mine ? new Date(mine.released).toLocaleString(localeOf(lang), { dateStyle: 'short', timeStyle: 'short' }) : '';
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
