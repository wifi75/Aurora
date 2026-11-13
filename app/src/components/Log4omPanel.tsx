import { useQuery, useQueryClient } from '@tanstack/react-query';
import { AddQso } from '../pages/ExportPage';
import { useEffect, useLayoutEffect, useRef, useState, memo } from 'react';
import { getLog4om, getLogQso, type LogQso } from '../api/client';
import { LogEditDialog } from './LogEditDialog';
import { toast } from '../lib/toast';
import { useT } from '../i18n';
import { usePrefs } from '../lib/prefs';
import { useStation } from '../state/station';
import { playAlert } from '../lib/voice';
import { ringState, soundFor } from '../lib/soundAssign';
import type { Log4omQso } from '../api/client';
import { ColumnsMenu, MovableColGroup, MovableTh, useMovableColumns } from './MovableColumns';
import '../styles/log-grid.css';

/** Colonne del log: si spostano e si allargano trascinando i titoli, come nel DX Cluster; ordine e larghezze restano su questo dispositivo. */
export const LOG_COLUMNS = ['date', 'call', 'band', 'mode', 'rst', 'country'] as const;
type LogCol = (typeof LOG_COLUMNS)[number];
/** Larghezze di partenza in em; il paese prende lo spazio che avanza. */
const LOG_EM: Record<LogCol, number> = { date: 6.6, call: 7, band: 3.8, mode: 3.8, rst: 4.8, country: 4 };
export const LOG_COLS_KEY = 'aurora.log.cols';
const LOG_LABEL = { date: 'log.col.date', call: 'log.col.call', band: 'log.col.band', mode: 'log.col.mode', rst: 'log.col.rst', country: 'log.col.country' } as const;

/** Una cella del log: sa a quale colonna appartiene, cosi' segue la colonna quando cambia posto. */
function logCell(q: Log4omQso, c: LogCol) {
  switch (c) {
    case 'date':
      // giorno/mese e ora, senza l'anno: la data intera nel suggerimento
      return (
        <td key={c} className="data log4om-date lg-c--date" title={q.date.replace('Z', '').slice(0, 16)}>
          {`${q.date.slice(8, 10)}/${q.date.slice(5, 7)} ${q.date.slice(11, 16)}`}
        </td>
      );
    case 'call':
      return <td key={c} className="data log4om-call lg-c--call">{q.call}</td>;
    case 'band':
      return <td key={c} className="data lg-c--band">{q.band}</td>;
    case 'mode':
      return <td key={c} className="data lg-c--mode">{q.mode}</td>;
    case 'rst':
      return <td key={c} className="data log4om-rst lg-c--rst">{[q.rst_sent, q.rst_rcvd].filter(Boolean).join('/')}</td>;
    case 'country':
      return <td key={c} className="log4om-country lg-c--country" title={q.country}>{q.country}</td>;
  }
}

/** Effetto del QSO appena messo a log: aurora che scorre dentro la riga e scie viola sul bordo per 5 s, poi si dissolve (vedi `.qso-fx`). */
const FLASH_MS = 6600;

/** Gli ultimi QSO del log di Log4OM2 (sola lettura): quanti, si sceglie in Impostazioni → Generali. */
function Log4omPanelInner() {
  const t = useT();
  const count = usePrefs((s) => s.prefs.log4omCount);
  const log = useQuery({ queryKey: ['log4om', count], queryFn: () => getLog4om(count), refetchInterval: 10_000, retry: false });
  const cols = useMovableColumns({ storageKey: LOG_COLS_KEY, columns: LOG_COLUMNS, defaultEm: LOG_EM, elastic: 'country' });
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
    // si riconosce dal nominativo (la prima riga con quel nominativo): l'ora scritta nel log puo' essere quella di fine QSO, un minuto
    // dopo quella dell'evento, e con l'ora la riga non si trovava (2026-11-09, BG4UCZ: niente animazione)
    setFlash(lastLogged.call.toUpperCase());
    // il suono scelto in Impostazioni > Avvisi, «QSO a log» (di fabbrica la fanfara; «Nessun suono» lo spegne)
    const st = ringState(false);
    const id = soundFor('qso_logged', st);
    if (id) playAlert(id, st.volume, { event: 'qso_logged', call: lastLogged.call });
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

  // doppio clic su un QSO del log interno di Aurora: si apre la finestra per correggerlo o cancellarlo (il log di Log4OM2 e' in sola lettura)
  const [editing, setEditing] = useState<LogQso | null>(null);
  const [adding, setAdding] = useState(false);
  const readOnly = log.data?.source === 'log4om';
  const edit = (q: Log4omQso) => {
    if (readOnly) {
      toast(t('log.readonly4om'), 'info');
      return;
    }
    if (q.id === undefined) return;
    getLogQso(q.id)
      .then(setEditing)
      .catch(() => toast(t('exp.edit.gone'), 'error'));
  };

  if (log.isError) return <p className="muted log4om-note">{t('log.unavailable')}</p>;
  if (!log.data) return <p className="muted log4om-note">…</p>;
  // log vuoto: le intestazioni delle colonne restano e sotto ci sono «Aggiungi QSO» e «Importa»; prima c'era solo il riquadro «Il log e'
  // vuoto» e un log nuovo non diceva come aggiungere un QSO a mano
  const empty = log.data.qsos.length === 0;
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
      {adding ? (
        <div className="logqso-backdrop" onClick={(e) => e.target === e.currentTarget && setAdding(false)}>
          <div className="logqso-dialog" role="dialog" aria-modal="true" aria-label={t('exp.add')}>
            <h3>{t('exp.add')}</h3>
            <AddQso onCancel={() => setAdding(false)} onDone={() => { setAdding(false); void client.invalidateQueries({ queryKey: ['log4om'] }); }} />
          </div>
        </div>
      ) : null}
      <table ref={cols.tableRef} className="log4om-table log4om-grid mv-grid" style={cols.tableStyle}>
        <MovableColGroup cols={cols} />
        <thead onContextMenu={cols.openMenu}>
          <tr>
            {cols.visible.map((c, i) => (
              <MovableTh key={c} cols={cols} col={c} index={i} className={`lg-c--${c}`} title={c === 'rst' ? t('tip.ui.colRst') : undefined}>
                {t(LOG_LABEL[c])}
              </MovableTh>
            ))}
          </tr>
        </thead>
        <tbody>
          {empty ? (
            <tr className="log4om-empty">
              <td colSpan={cols.visible.length}>
                <span className="muted">{t('log.noQsoYet')}</span>
                {readOnly ? null : (
                  <button type="button" className="glass chip" onClick={() => setAdding(true)}>
                    {t('exp.add')}
                  </button>
                )}
                <a className="glass chip" href="#/impostazioni/files">
                  {t('log.importAdif')}
                </a>
              </td>
            </tr>
          ) : null}
          {shownQsos.map((q, i) => (
            <tr
              key={`${q.date}-${q.call}-${i}`}
              className={flash !== null && i === shownQsos.findIndex((x) => x.call.toUpperCase() === flash) ? 'log4om-row--flash' : undefined}
              title={readOnly ? undefined : 'id' in q && q.id !== undefined ? t('log.editTip') : undefined}
              onDoubleClick={() => edit(q)}
            >
              {cols.visible.map((c) => logCell(q, c))}
            </tr>
          ))}
        </tbody>
      </table>
      <ColumnsMenu cols={cols} />
      {readOnly ? <p className="muted log4om-note log4om-readonly">{t('log.readonly4om')}</p> : null}
      {editing ? <LogEditDialog qso={editing} onClose={() => setEditing(null)} /> : null}
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
