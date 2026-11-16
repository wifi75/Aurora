import { Icon } from '../Icon';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useRef, useState } from 'react';
import { browserStorage } from '../../lib/theme';
import { ClusterPanel } from '../ClusterPanel';
import { SpotDialog } from './SpotDialog';
import { getLog4om, type ClusterSpot, type Log4omQso } from '../../api/client';
import type { TextContestScore } from '../../api/contestTesto';
import { ClusterOptionsDialog } from './ClusterOptionsDialog';
import { useClusterOptions } from '../../lib/clusterOptions';
import { bandColor } from '../../lib/mapColors';
import { useT } from '../../i18n';
import { formatSerial } from '../../lib/contestSerial';
import { ColumnsMenu, MovableColGroup, MovableTh, useMovableColumns } from '../MovableColumns';

export interface Qso {
  call: string;
  name: string;
  qth: string;
  wwloc: string;
  notes: string;
  /** Scambio da mandare (contest; vuoto = il numero progressivo) e ricevuto. */
  nrs?: string;
  nrr?: string;
  /** Il paese del nominativo (da cty.dat, con la ricerca): solo da vedere, il log lo ricava da se'. */
  country?: string;
}

interface Props {
  qso: Qso;
  rsts: string;
  rstr: string;
  modeName: string;
  /** Frequenza della radio in Hz (0 se non nota). */
  dialHz: number;
  /** Frequenza audio del segnale ricevuto (Hz): serve alla frequenza dello spot. */
  audioHz: number;
  /** Clic su uno spot del DX Cluster: porta la radio e, con «up N», lo split (`splitDx`: subito dopo la radio va in Split DX). */
  onSpot(spot: ClusterSpot, splitDx: boolean): Promise<void>;
  onQso(patch: Partial<Qso>): void;
  onRst(patch: { rsts?: string; rstr?: string }): void;
  onSave(): void;
  onClear(): void;
  /** Il numero progressivo del contest (<QSONR>) e come correggerlo. */
  serial: number;
  onSerial(n: number): void;
  /** C'e' un contest attivo (gara o contest dei modi testuali): solo allora si vedono progressivo, NRS e NRR. */
  contestOn: boolean;
  /** Larghezza del log in px e come cambiarla (barra di trascinamento sul bordo sinistro). */
  width: number;
  onWidth(px: number): void;
  /** Vista fissa (contest SSB a tre colonne): solo il Log o solo il DX Cluster, senza linguette, senza riga del QSO in corso e senza barra in fondo. */
  view?: 'log' | 'cluster';
  /** Con un contest: la colonna DUPE / Mult nel log (doppi dal log stesso, moltiplicatori nuovi dalle statistiche del contest). */
  marks?: LogMarks;
}

/** Come segnare doppi e moltiplicatori nelle righe del log. `noBand`/`noMode` come `dupe_no_band`/`dupe_no_mode` del contest. */
export interface LogMarks {
  noBand: boolean;
  noMode: boolean;
  recenti: TextContestScore['recenti'];
  /** Il QSO e' del contest (finestra di date, categoria, dopo l'attivazione se il contest non ha date)? Doppi e moltiplicatori valgono solo per questi. */
  inSet: (q: Log4omQso) => boolean;
  /** Si vede solo il log del contest (di base) o tutto il log. */
  only: boolean;
  onToggle(): void;
}

/** Le colonne del log del contest: numero progressivo del contest, DUPE / Mult, ora, nominativo, banda, frequenza e i due rapporti. */
const CONTEST_COLS: readonly LogCol[] = ['num', 'mark', 'utc', 'call', 'band', 'freq', 'rstS', 'rstR']

/** Per ogni QSO (il piu' recente in testa) se e' un doppio: c'e' un QSO piu' vecchio con lo stesso nominativo (e banda, e modo). */
export function dupeFlags(qsos: readonly Pick<Log4omQso, 'call' | 'band' | 'mode'>[], noBand: boolean, noMode: boolean): boolean[] {
  const seen = new Set<string>();
  const out: boolean[] = new Array<boolean>(qsos.length).fill(false);
  for (let i = qsos.length - 1; i >= 0; i--) {
    const q = qsos[i];
    if (!q) continue;
    const key = `${q.call.toUpperCase()}|${noBand ? '' : q.band}|${noMode ? '' : q.mode.toUpperCase()}`;
    out[i] = seen.has(key);
    seen.add(key);
  }
  return out;
}

export type LogCol = 'num' | 'mark' | 'mode' | 'band' | 'freq' | 'date' | 'utc' | 'call' | 'name' | 'qth' | 'country' | 'grid' | 'rstS' | 'rstR';
const ALL_COLS: readonly LogCol[] = ['num', 'mark', 'mode', 'band', 'freq', 'date', 'utc', 'call', 'name', 'qth', 'country', 'grid', 'rstS', 'rstR'];
/** Le colonne del log: numero, modo, frequenza, data, UTC, nominativo, nome, QTH e i due rapporti. */
const DEFAULT_COLS: readonly LogCol[] = ['num', 'mode', 'freq', 'date', 'utc', 'call', 'name', 'qth', 'rstS', 'rstR'];
/** Le colonne che si scelgono con la matita: «mark» c'e' solo con un contest. */
const PICK_COLS: readonly LogCol[] = ALL_COLS.filter((c) => c !== 'mark');
const COLS_KEY = 'aurora.digi.logcols2';
/** Quanti QSO si leggono dal log (il massimo che il core da': i piu' recenti). */
const LOG_ROWS = 500;

function savedCols(): LogCol[] {
  try {
    const raw = JSON.parse(browserStorage()?.getItem(COLS_KEY) ?? 'null') as unknown;
    if (Array.isArray(raw)) {
      const ok = ALL_COLS.filter((c) => raw.includes(c));
      if (ok.length > 0) return ok;
    }
  } catch {
    // senza archivio restano le colonne di base
  }
  return [...DEFAULT_COLS];
}

/** Larghezza di partenza di ogni colonna, in em: cresce da sola con la dimensione del carattere. */
const DEFAULT_EM: Record<LogCol, number> = { num: 4.2, mark: 7, mode: 4.6, band: 3.6, freq: 8.2, date: 7.6, utc: 6.4, call: 8.6, name: 13, qth: 12, country: 12, grid: 5.6, rstS: 5, rstR: 5 };
/** Ordine e larghezze delle colonne (si spostano e si allargano trascinando i titoli, come nel DX Cluster). */
export const GRID_KEY = 'aurora.digi.log.grid';
/** Dove stavano le sole larghezze prima delle colonne spostabili: si riprendono una volta. */
export const OLD_WIDTHS_KEY = 'aurora.digi.logw3';

/** Le larghezze scelte prima che le colonne si potessero spostare passano, una volta sola, nella disposizione nuova. */
export function migrateWidths(): void {
  try {
    const store = browserStorage();
    if (!store || store.getItem(GRID_KEY) !== null) return;
    const old = JSON.parse(store.getItem(OLD_WIDTHS_KEY) ?? 'null') as unknown;
    if (old && typeof old === 'object') store.setItem(GRID_KEY, JSON.stringify({ order: ALL_COLS, widths: old }));
  } catch {
    // senza archivio restano le larghezze di partenza
  }
}

/** Frequenza in kHz interi, senza i decimali dopo la virgola (7048 invece di 7048,783). */
const khz = (hz: number) => (hz > 0 ? String(Math.round(hz / 1000)) : '');
/** `2023-03-19` -> `19/03/2023`. */
const dmy = (d: string) => (/^\d{4}-\d{2}-\d{2}$/.test(d) ? `${d.slice(8, 10)}/${d.slice(5, 7)}/${d.slice(0, 4)}` : d);

const COL_LABEL = {
  num: 'digi.log.num', mark: 'ssbc.logMark', mode: 'digi.log.mode', band: 'digi.log.band', freq: 'digi.log.freq', date: 'digi.log.date', utc: 'digi.log.utc',
  call: 'digi.log.call', name: 'digi.log.name', qth: 'digi.log.qth', country: 'digi.log.country', grid: 'digi.log.grid', rstS: 'digi.log.rsts', rstR: 'digi.log.rstr',
} as const;

/** Il colore del modo nell'etichetta. */
const MODE_COLORS: Record<string, string> = { RTTY: '#60a5fa', PSK31: '#c084fc', PSK63: '#c084fc', PSK: '#c084fc', CW: '#fbbf24', FT8: '#34d399', FT4: '#2dd4bf', SSTV: '#fb923c' };
const modeColor = (m: string) => MODE_COLORS[m.toUpperCase()] ?? '#94a3b8';

/** Rapporto buono (verde), medio (ambra) o debole (rosso): 59 o piu' e dB a -10 o meglio sono buoni. */
function rstClass(v: string): string {
  const n = Number(v);
  if (!Number.isFinite(n) || v.trim() === '') return '';
  if (/^[+-]d{1,2}$/.test(v.trim())) return n >= -10 ? 'rst-good' : n >= -18 ? 'rst-mid' : 'rst-low';
  const r = Math.floor(n / 10);
  return r >= 59 ? 'rst-good' : r >= 45 ? 'rst-mid' : 'rst-low';
}

function cell(q: Log4omQso, nr: number, c: LogCol, mark: React.ReactNode = null): React.ReactNode {
  switch (c) {
    case 'num': return nr;
    case 'mark': return mark;
    case 'mode': return q.mode === '' ? '' : <span className="digi-mode-badge" style={{ background: modeColor(q.mode) }}>{q.mode}</span>;
    case 'band': return q.band;
    case 'freq': return khz(q.freq_hz ?? 0);
    case 'date': return dmy(q.date.slice(0, 10));
    case 'utc': return q.date.slice(11, 19);
    case 'call': return <b>{q.call}</b>;
    case 'name': return q.name ?? '';
    case 'qth': return q.qth ?? '';
    case 'country': return q.country;
    case 'grid': return q.grid;
    case 'rstS': return <span className={rstClass(q.rst_sent)}>{q.rst_sent}</span>;
    case 'rstR': return <span className={rstClass(q.rst_rcvd)}>{q.rst_rcvd}</span>;
  }
}

/**
 * Il log di lato: la riga azzurra del QSO in corso (si scrive sul posto) e sotto i QSO del log, il piu' recente in testa.
 * Nell'altra linguetta c'e' il DX Cluster, con il QSO in corso in un riquadro sotto.
 */
export function DigiLog({ qso, rsts, rstr, modeName, dialHz, audioHz, onSpot, onQso, onRst, onSave, onClear, serial, onSerial, contestOn, width, onWidth, view, marks }: Props) {
  const t = useT();
  const [filter, setFilter] = useState('');
  const [cols, setCols] = useState<LogCol[]>(savedCols);
  const [picking, setPicking] = useState(false);
  const body = useRef<HTMLDivElement>(null);
  // prima di leggere la disposizione: le larghezze di una volta passano nella chiave nuova
  useState(migrateWidths);
  // le colonne scelte con la matita si vedono, le altre restano al loro posto nell'ordine; la riga azzurra del QSO in corso (campi da
  // scrivere) non conta quando si adatta una colonna al contenuto
  const grid = useMovableColumns({
    storageKey: GRID_KEY,
    columns: ALL_COLS,
    defaultEm: DEFAULT_EM,
    hidden: ALL_COLS.filter((c) => (marks ? !CONTEST_COLS.includes(c) : c === 'mark' || !cols.includes(c))),
    fitCells: (_c, i) => body.current?.querySelectorAll(`thead tr > :nth-child(${i + 1}), tbody tr:not(.digi-log-cur) > :nth-child(${i + 1})`) ?? [],
  });
  const [spotOpen, setSpotOpen] = useState(false);
  const [optionsOpen, setOptionsOpen] = useState(false);
  const [clusterOptions, setClusterOptions] = useClusterOptions();
  // linguette: DX Cluster o Log (la scelta resta ricordata)
  const [tabState, setTabState] = useState<'cluster' | 'log'>(() => (browserStorage()?.getItem('aurora.digi.tab') === 'log' ? 'log' : 'cluster'));
  const tab = view ?? tabState;
  const setTab = (next: 'cluster' | 'log') => {
    if (view) return;
    setTabState(next);
    try {
      browserStorage()?.setItem('aurora.digi.tab', next);
    } catch {
      // la scelta vale comunque finche' la pagina e' aperta
    }
  };
  const pop = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!picking) return;
    const out = (e: PointerEvent) => {
      if (!pop.current?.contains(e.target as Node)) setPicking(false);
    };
    document.addEventListener('pointerdown', out);
    return () => document.removeEventListener('pointerdown', out);
  }, [picking]);
  /** Il corpo del carattere della tabella in px: le larghezze sono in em e seguono la dimensione scelta nelle Impostazioni. */
  const emPx = () => parseFloat(getComputedStyle(body.current?.querySelector('table') ?? document.body).fontSize) || 12;
  /**
   * Tutte le colonne in vista larghe quanto il loro contenuto (mai meno di 6/10 della larghezza di partenza); le colonne nascoste tengono la
   * larghezza scelta prima (`setWidths` sostituisce tutto, quindi si parte da quelle gia' salvate).
   */
  const autoFitAll = () => {
    const table = body.current?.querySelector('table');
    if (!table) return;
    const next: Partial<Record<LogCol, number>> = { ...grid.layout.widths };
    grid.visible.forEach((c, i) => {
      let max = 0;
      table.querySelectorAll(`tr:not(.digi-log-cur) > :nth-child(${i + 1})`).forEach((el) => {
        max = Math.max(max, (el as HTMLElement).scrollWidth);
      });
      next[c] = Math.max(max / emPx() + 0.3, DEFAULT_EM[c] * 0.6);
    });
    grid.setWidths(next);
  };
  const resetWidths = () => grid.setWidths({});
  const toggleCol = (c: LogCol) => {
    const next = cols.includes(c) ? cols.filter((x) => x !== c) : ALL_COLS.filter((x) => x === c || cols.includes(x));
    if (next.length === 0) return;
    setCols(next);
    try {
      browserStorage()?.setItem(COLS_KEY, JSON.stringify(next));
    } catch {
      // la scelta vale comunque finche' la pagina e' aperta
    }
  };
  // lo stesso log del pannello Log (Log4OM2 o il log di Aurora): il totale e gli ultimi QSO, il piu' recente in testa
  const log = useQuery({ queryKey: ['log4om', LOG_ROWS], queryFn: () => getLog4om(LOG_ROWS), retry: false, refetchInterval: 10_000 });
  // dal log, il piu' recente in testa (l'elenco arriva gia' cosi'): i primi 300
  const total = log.data?.total ?? 0;
  const rows = useMemo(() => {
    // il numero del QSO e' la sua posizione nel log (l'ultimo = il totale), anche cercando un nominativo
    const qsos = log.data?.qsos ?? [];
    // con un contest: i QSO del contest (finestra, categoria, attivazione) sono numerati 1, 2, 3... e solo fra loro si cercano doppi e moltiplicatori
    const mine = marks ? qsos.map((q) => marks.inSet(q)) : [];
    const subset = marks ? qsos.filter((_, i) => mine[i]) : [];
    const dupesSub = marks ? dupeFlags(subset, marks.noBand, marks.noMode) : [];
    let k = 0;
    const all: { q: Log4omQso; nr: number; dupe: boolean; mult: string }[] = [];
    qsos.forEach((q, i) => {
      if (marks && !mine[i]) {
        if (!marks.only) all.push({ q, nr: total - i, dupe: false, mult: '' });
        return;
      }
      const j = k++;
      const nuovi = marks ? (marks.recenti.find((r) => r.call.toUpperCase() === q.call.toUpperCase() && r.band === q.band)?.nuovi ?? []) : [];
      all.push({ q, nr: marks ? subset.length - j : total - i, dupe: marks ? dupesSub[j] === true : false, mult: nuovi.map((m) => m.slice(m.indexOf(':') + 1)).join(' ') });
    });
    const f = filter.trim().toUpperCase();
    return (f === '' ? all : all.filter((x) => x.q.call.includes(f))).slice(0, 300);
  }, [log.data, filter, total, marks]);
  const contestTotal = marks ? (log.data?.qsos ?? []).filter((q) => marks.inSet(q)).length : 0;
  const nextNum = total + 1;
  const now = new Date();
  const nowDate = `${String(now.getUTCDate()).padStart(2, '0')}/${String(now.getUTCMonth() + 1).padStart(2, '0')}/${now.getUTCFullYear()}`;
  const nowTime = now.toISOString().slice(11, 19);

  // la tabella parte dall'alto: la riga in corso e l'ultimo QSO si vedono sempre
  useEffect(() => {
    const el = body.current;
    if (el) el.scrollTop = 0;
  }, [rows.length, tab]);

  const input = (value: string, max: number, onChange: (v: string) => void, upper = false) => (
    <input className="digi-cur-in" value={value} maxLength={max} onChange={(e) => onChange(upper ? e.target.value.toUpperCase() : e.target.value)} />
  );
  const current = (c: LogCol): React.ReactNode => {
    switch (c) {
      case 'num': return nextNum;
      case 'mark': return '';
      case 'mode': return modeName;
      case 'band': return '';
      case 'freq': return khz(dialHz > 0 ? dialHz + Math.round(audioHz) : 0);
      case 'date': return nowDate;
      case 'utc': return nowTime;
      case 'call': return input(qso.call, 16, (v) => onQso({ call: v }), true);
      case 'name': return input(qso.name, 30, (v) => onQso({ name: v }));
      case 'qth': return input(qso.qth, 40, (v) => onQso({ qth: v }));
      case 'country': return qso.country ?? '';
      case 'grid': return input(qso.wwloc, 8, (v) => onQso({ wwloc: v }), true);
      case 'rstS': return input(rsts, 4, (v) => onRst({ rsts: v }));
      case 'rstR': return input(rstr, 4, (v) => onRst({ rstr: v }));
    }
  };

  return (
    <aside className="digi-log" aria-label={t('digi.log.title')}>
      {view ? null : <div
        className="digi-log-grip"
        role="separator"
        aria-orientation="vertical"
        title={t('digi.log.resize')}
        onPointerDown={(e) => {
          e.preventDefault();
          const startX = e.clientX;
          const startW = width;
          const move = (m: PointerEvent) => onWidth(startW + (startX - m.clientX));
          const up = () => {
            window.removeEventListener('pointermove', move);
            window.removeEventListener('pointerup', up);
          };
          window.addEventListener('pointermove', move);
          window.addEventListener('pointerup', up);
        }}
      />}
      <div className="digi-log-tabs" role="tablist">
        {view !== 'log' ? (
          <button type="button" role="tab" aria-selected={tab === 'cluster'} className={tab === 'cluster' ? 'on' : ''} onClick={() => setTab('cluster')}>
            {t('digi.tab.cluster')}
          </button>
        ) : null}
        {view !== 'cluster' ? (
          <button type="button" role="tab" aria-selected={tab === 'log'} className={tab === 'log' ? 'on' : ''} onClick={() => setTab('log')}>
            {t('digi.log.title')}
          </button>
        ) : null}
        {tab === 'cluster' ? (
          <span className="digi-log-tabtools">
            <button type="button" className="sstv-btn glass" title={t('digi.spot.tip')} onClick={() => setSpotOpen(true)}>
              {t('ctl.spot')}
            </button>
            <button type="button" className="digi-tool icon-btn icon-btn--sm" title={t('cop.tip')} aria-label={t('cop.tip')} onClick={() => setOptionsOpen(true)}>
              <Icon name="gear" />
            </button>
          </span>
        ) : null}
        {tab === 'log' ? (
          <span className="digi-log-tabtools">
            <input className="digi-log-search" placeholder={t('digi.log.search')} aria-label={t('digi.log.search')} value={filter} onChange={(e) => setFilter(e.target.value)} />
            <span className="digi-colwrap" ref={pop}>
              <button type="button" className="digi-tool" title={t('digi.log.cols')} aria-label={t('digi.log.cols')} aria-expanded={picking} onClick={() => setPicking(!picking)}>
                ✎
              </button>
              {picking ? (
                <div className="digi-colpop" role="dialog" aria-label={t('digi.log.cols')}>
                  {PICK_COLS.map((c) => (
                    <label key={c}>
                      <input type="checkbox" checked={cols.includes(c)} onChange={() => toggleCol(c)} /> {t(COL_LABEL[c])}
                    </label>
                  ))}
                  <button type="button" className="digi-colpop-btn" onClick={autoFitAll}>
                    {t('digi.log.fitAll')}
                  </button>
                  <button type="button" className="digi-colpop-btn" onClick={resetWidths}>
                    {t('digi.log.resetWidths')}
                  </button>
                  <p>{t('digi.log.colsHint')}</p>
                </div>
              ) : null}
            </span>
          </span>
        ) : null}
      </div>

      {tab === 'cluster' ? (
        <>
          <div className="digi-log-cluster">
            <ClusterPanel onSpot={onSpot} options={clusterOptions} />
          </div>
        </>
      ) : (
        <>
          <div className="digi-log-table digi-log-table--completa" ref={body}>
            <table ref={grid.tableRef} className="mv-grid" style={grid.tableStyle}>
              <MovableColGroup cols={grid} />
              <thead onContextMenu={grid.openMenu}>
                <tr>
                  {grid.visible.map((c, i) => (
                    <MovableTh key={c} cols={grid} col={c} index={i}>
                      {t(COL_LABEL[c])}
                    </MovableTh>
                  ))}
                </tr>
              </thead>
              <tbody>
                {view ? null : (
                  <tr className="digi-log-cur">
                    {grid.visible.map((c) => (
                      <td key={c}>{current(c)}</td>
                    ))}
                  </tr>
                )}
                {rows.map(({ q, nr, dupe, mult }) => (
                  <tr key={nr} className={dupe ? 'digi-log-dupe' : undefined} style={{ '--b': bandColor(q.band) } as React.CSSProperties}>
                    {grid.visible.map((c) => (
                      <td key={c}>{cell(q, nr, c, dupe ? <b className="digi-mark-dupe">DUPE</b> : mult !== '' ? <b className="digi-mark-mult">× {mult}</b> : null)}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
            <ColumnsMenu cols={grid} />
            {rows.length === 0 ? <p className="digi-log-empty">{t('digi.log.empty')}</p> : null}
          </div>
          {view && marks ? (
            <div className="digi-log-bar ssbc-logbar">
              <span className="ssbc-logtotal">{t('ssbc.logTotal', { n: contestTotal })}</span>
              <button type="button" className="digi-tool" aria-pressed={marks.only} title={t('ssbc.logToggleTip')} onClick={marks.onToggle}>
                {marks.only ? t('ssbc.logAll') : t('ssbc.logOnly')}
              </button>
            </div>
          ) : null}
          {view ? null : <div className="digi-log-bar">
            {contestOn ? (
              <>
            <label className="digi-log-ex" title={t('digi.log.serialTip')}>
              {t('digi.log.serial')}
              <input className="digi-cur-in" type="number" min={1} max={99999} style={{ width: '5em' }} value={serial} onChange={(e) => onSerial(Number(e.target.value))} />
            </label>
            <button type="button" className="digi-tool" title={t('digi.log.serialReset')} aria-label={t('digi.log.serialReset')} onClick={() => onSerial(1)}>
              ↺
            </button>
            <label className="digi-log-ex" title={t('digi.log.nrsTip')}>
              NRS
              <input className="digi-cur-in" style={{ width: '5em' }} maxLength={12} value={qso.nrs ?? ''} placeholder={formatSerial(serial)} onChange={(e) => onQso({ nrs: e.target.value.toUpperCase() })} />
            </label>
            <label className="digi-log-ex" title={t('digi.log.nrrTip')}>
              NRR
              <input className="digi-cur-in" style={{ width: '5em' }} maxLength={12} value={qso.nrr ?? ''} onChange={(e) => onQso({ nrr: e.target.value.toUpperCase() })} />
            </label>
              </>
            ) : null}
            <button type="button" className="digi-tool" title={t('digi.log.save')} aria-label={t('digi.log.save')} onClick={onSave}>
              💾
            </button>
            <button type="button" className="digi-tool digi-tool--red" title={t('digi.log.clear')} aria-label={t('digi.log.clear')} onClick={onClear}>
              ✕
            </button>
          </div>}
        </>
      )}
      {optionsOpen ? <ClusterOptionsDialog options={clusterOptions} onChange={setClusterOptions} onClose={() => setOptionsOpen(false)} /> : null}
      {spotOpen ? <SpotDialog call={qso.call} dialHz={dialHz} audioHz={audioHz} modeName={modeName} onClose={() => setSpotOpen(false)} /> : null}
    </aside>
  );
}
