import { useQuery, useQueryClient } from '@tanstack/react-query';
import { SetupCta } from './SetupCta';
import { useEffect, useLayoutEffect, useRef, useState, memo } from 'react';
import { getLog4om } from '../api/client';
import { useT } from '../i18n';
import { usePrefs } from '../lib/prefs';
import { useStation } from '../state/station';
import { playSound } from '../lib/soundBank';
import { ringState } from '../lib/soundAssign';

/** Effetto del QSO appena messo a log: aurora che scorre dentro la riga e scie viola sul bordo per 5 s, poi si dissolve (vedi `.qso-fx`). */
const FLASH_MS = 6600;

/** Gli ultimi QSO del log di Log4OM2 (sola lettura): quanti, si sceglie in Impostazioni → Generali. */
function Log4omPanelInner() {
  const t = useT();
  const count = usePrefs((s) => s.prefs.log4omCount);
  const log = useQuery({ queryKey: ['log4om', count], queryFn: () => getLog4om(count), refetchInterval: 10_000, retry: false });
  const client = useQueryClient();
  // appena un QSO viene messo a log il pannello si rilegge subito, senza aspettare il giro dei 10 secondi
  const logged = useStation((st) => st.qsosLogged);
  useEffect(() => {
    if (logged <= 0) return;
    // il QSO arriva nel database di Log4OM qualche istante dopo (ricerca del nome, invio, scrittura): si rilegge subito e poi ancora
    // dopo 1,5, 4 e 8 secondi, invece di aspettare il giro dei 10 secondi
    void client.invalidateQueries({ queryKey: ['log4om'] });
    const timers = [1500, 4000, 8000].map((ms) => window.setTimeout(() => void client.invalidateQueries({ queryKey: ['log4om'] }), ms));
    return () => timers.forEach((id) => window.clearTimeout(id));
  }, [logged, client]);
  // il QSO appena messo a log compare subito in cima (dall'evento di Aurora), anche prima che Log4OM lo scriva; quando arriva dal log
  // vero si mostra quello. Dopo 2 minuti quello provvisorio sparisce comunque.
  const justLogged = useStation((st) => st.justLogged);
  const shownQsos = (() => {
    const real = log.data?.qsos ?? [];
    const day = new Date().toISOString().slice(0, 10);
    const fresh = justLogged
      .filter((j) => Date.now() - j.at < 120_000)
      .filter((j) => !real.slice(0, 5).some((q) => q.call.toUpperCase() === j.call.toUpperCase() && q.band === j.band))
      .map((j) => {
        const sec = Math.floor((j.time_ms % 86_400_000) / 1000);
        const hh = String(Math.floor(sec / 3600)).padStart(2, '0');
        const mm = String(Math.floor((sec % 3600) / 60)).padStart(2, '0');
        return { date: `${day}T${hh}:${mm}:00Z`, call: j.call, band: j.band, mode: j.mode, rst_sent: j.rst_sent, rst_rcvd: j.rst_rcvd, country: '', grid: '' };
      });
    return [...fresh, ...real].slice(0, Math.max(real.length, count));
  })();
  // un QSO nuovo in cima: tutta la riga lampeggia tre volte, piano
  const [flash, setFlash] = useState<string | null>(null);
  // tutto nello stesso istante, appena il QSO va a log (dall'evento di Aurora, non quando Log4OM lo scrive): la riga compare in cima,
  // lampeggia e suona. La riga di Log4OM che arriva dopo per lo stesso QSO non lampeggia una seconda volta.
  const lastLogged = useStation((st) => st.justLogged[0]);
  useEffect(() => {
    if (!lastLogged || Date.now() - lastLogged.at > 5000) return;
    const sec = Math.floor((lastLogged.time_ms % 86_400_000) / 1000);
    const hhmm = `${String(Math.floor(sec / 3600)).padStart(2, '0')}:${String(Math.floor((sec % 3600) / 60)).padStart(2, '0')}`;
    setFlash(`${lastLogged.call.toUpperCase()}|${hhmm}`);
    const st = ringState(false);
    if (st.enabled) playSound('fanfare', st.volume);
    const timer = window.setTimeout(() => setFlash(null), FLASH_MS);
    return () => window.clearTimeout(timer);
  }, [lastLogged]);


  // l'aurora e le scie stanno in uno strato sotto la riga (le celle di una tabella non hanno uno sfondo continuo da un capo all'altro):
  // lo strato segue la riga finche' l'effetto e' acceso, anche se il pannello scorre o cambia larghezza
  const boxRef = useRef<HTMLDivElement | null>(null);
  const [fx, setFx] = useState<{ top: number; left: number; width: number; height: number } | null>(null);
  useLayoutEffect(() => {
    if (flash === null) {
      setFx(null);
      return;
    }
    const place = () => {
      const box = boxRef.current;
      const row = box?.querySelector<HTMLTableRowElement>('tr.log4om-row--flash');
      if (!box || !row) return setFx(null);
      const b = box.getBoundingClientRect();
      const r = row.getBoundingClientRect();
      setFx({ top: r.top - b.top + box.scrollTop, left: r.left - b.left + box.scrollLeft, width: r.width, height: r.height });
    };
    place();
    const id = window.setInterval(place, 250);
    return () => window.clearInterval(id);
  }, [flash, log.data]);

  if (log.isError) return <p className="muted log4om-note">{t('log.unavailable')}</p>;
  if (!log.data) return <p className="muted log4om-note">…</p>;
  if (log.data.qsos.length === 0) return <SetupCta className="setup-cta--log" href="#/impostazioni/reports" title="cta.log.none" action="cta.log.go" />;
  return (
    <div className="log4om" ref={boxRef}>
      {fx ? (
        <div className="qso-fx" aria-hidden="true" style={{ top: fx.top, left: fx.left, width: fx.width, height: fx.height }}>
          <i className="qso-fx-aurora" />
          <i className="qso-fx-trail qso-fx-trail--top" />
          <i className="qso-fx-trail qso-fx-trail--bottom" />
          <b className="qso-fx-badge">{t('log.loggedBadge')}</b>
        </div>
      ) : null}
      <table className="log4om-table">
        <thead>
          <tr>
            <th>{t('log.col.date')}</th>
            <th>{t('log.col.call')}</th>
            <th>{t('log.col.band')}</th>
            <th>{t('log.col.mode')}</th>
            <th title={t('tip.ui.colRst')}>{t('log.col.rst')}</th>
            <th>{t('log.col.country')}</th>
          </tr>
        </thead>
        <tbody>
          {shownQsos.map((q, i) => (
            <tr key={`${q.date}-${q.call}-${i}`} className={flash !== null && flash === `${q.call.toUpperCase()}|${q.date.slice(11, 16)}` ? 'log4om-row--flash' : undefined}>
              {/* giorno/mese e ora, senza l'anno: la data intera nel suggerimento */}
              <td className="data log4om-date" title={q.date.replace('Z', '').slice(0, 16)}>
                {`${q.date.slice(8, 10)}/${q.date.slice(5, 7)} ${q.date.slice(11, 16)}`}
              </td>
              <td className="data log4om-call">{q.call}</td>
              <td className="data">{q.band}</td>
              <td className="data">{q.mode}</td>
              <td className="data log4om-rst">{[q.rst_sent, q.rst_rcvd].filter(Boolean).join('/')}</td>
              <td className="log4om-country" title={q.country}>{q.country}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** Nella barra del pannello Log: quanti QSO (l'esportazione ADIF sta nella pagina Log del menu in alto). */
export function LogHeader() {
  const t = useT();
  const count = usePrefs((s) => s.prefs.log4omCount);
  const log = useQuery({ queryKey: ['log4om', count], queryFn: () => getLog4om(count), refetchInterval: 10_000, retry: false });
  return (
    <div className="dock-grip-extra dock-grip-extra--right" onPointerDown={(e) => e.stopPropagation()}>
      {log.data ? <span className="muted log4om-total">{t('log.total', { n: log.data.total })}</span> : null}
    </div>
  );
}

/** Senza proprieta': si ridisegna solo per i suoi dati, non ogni volta che si ridisegna la Stazione. */
export const Log4omPanel = memo(Log4omPanelInner);
