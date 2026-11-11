import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useRef, useState } from 'react';
import { browserStorage } from '../../lib/theme';
import { ClusterPanel } from '../ClusterPanel';
import { SpotDialog } from './SpotDialog';
import { getLog4om, type ClusterSpot, type Log4omQso } from '../../api/client';
import { ClusterOptionsDialog } from './ClusterOptionsDialog';
import { useClusterOptions } from '../../lib/clusterOptions';
import { bandColor } from '../../lib/mapColors';
import { useT } from '../../i18n';

export interface Qso {
  call: string;
  name: string;
  qth: string;
  wwloc: string;
  notes: string;
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
  /** Clic su uno spot del DX Cluster: porta la radio e, con «up N», lo split. */
  onSpot(spot: ClusterSpot): Promise<void>;
  onQso(patch: Partial<Qso>): void;
  onRst(patch: { rsts?: string; rstr?: string }): void;
  onSave(): void;
  onClear(): void;
  /** Larghezza del log in px e come cambiarla (barra di trascinamento sul bordo sinistro). */
  width: number;
  onWidth(px: number): void;
}

export type LogCol = 'num' | 'mode' | 'band' | 'freq' | 'date' | 'utc' | 'call' | 'name' | 'qth' | 'country' | 'grid' | 'rstS' | 'rstR';
const ALL_COLS: readonly LogCol[] = ['num', 'mode', 'band', 'freq', 'date', 'utc', 'call', 'name', 'qth', 'country', 'grid', 'rstS', 'rstR'];
/** Le colonne del log: numero, modo, frequenza, data, UTC, nominativo, nome, QTH e i due rapporti. */
const DEFAULT_COLS: readonly LogCol[] = ['num', 'mode', 'freq', 'date', 'utc', 'call', 'name', 'qth', 'rstS', 'rstR'];
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
const DEFAULT_EM: Record<LogCol, number> = { num: 4.2, mode: 4.6, band: 3.6, freq: 8.2, date: 7.6, utc: 6.4, call: 8.6, name: 13, qth: 12, country: 12, grid: 5.6, rstS: 5, rstR: 5 };
const WIDTHS_KEY = 'aurora.digi.logw3';
const MIN_EM = 2.5;
const MAX_EM = 48;

function savedWidths(): Partial<Record<LogCol, number>> {
  try {
    const raw = JSON.parse(browserStorage()?.getItem(WIDTHS_KEY) ?? 'null') as unknown;
    if (raw && typeof raw === 'object') {
      const out: Partial<Record<LogCol, number>> = {};
      for (const c of ALL_COLS) {
        const v = (raw as Record<string, unknown>)[c];
        if (typeof v === 'number' && Number.isFinite(v)) out[c] = Math.min(MAX_EM, Math.max(MIN_EM, v));
      }
      return out;
    }
  } catch {
    // senza archivio restano le larghezze di partenza
  }
  return {};
}

/** Frequenza in kHz con la virgola, come nel log (7042,988). */
const khz = (hz: number) => (hz > 0 ? (hz / 1000).toFixed(3).replace(/(\.\d)0+$/, '$1').replace('.', ',') : '');
/** `2023-03-19` -> `19/03/2023`. */
const dmy = (d: string) => (/^\d{4}-\d{2}-\d{2}$/.test(d) ? `${d.slice(8, 10)}/${d.slice(5, 7)}/${d.slice(0, 4)}` : d);

const COL_LABEL = {
  num: 'digi.log.num', mode: 'digi.log.mode', band: 'digi.log.band', freq: 'digi.log.freq', date: 'digi.log.date', utc: 'digi.log.utc',
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

function cell(q: Log4omQso, nr: number, c: LogCol): React.ReactNode {
  switch (c) {
    case 'num': return nr;
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
export function DigiLog({ qso, rsts, rstr, modeName, dialHz, audioHz, onSpot, onQso, onRst, onSave, onClear, width, onWidth }: Props) {
  const t = useT();
  const [filter, setFilter] = useState('');
  const [cols, setCols] = useState<LogCol[]>(savedCols);
  const [picking, setPicking] = useState(false);
  const [widths, setWidths] = useState<Partial<Record<LogCol, number>>>(savedWidths);
  const [spotOpen, setSpotOpen] = useState(false);
  const [optionsOpen, setOptionsOpen] = useState(false);
  const [clusterOptions, setClusterOptions] = useClusterOptions();
  // linguette: DX Cluster o Log (la scelta resta ricordata)
  const [tab, setTabState] = useState<'cluster' | 'log'>(() => (browserStorage()?.getItem('aurora.digi.tab') === 'log' ? 'log' : 'cluster'));
  const setTab = (next: 'cluster' | 'log') => {
    setTabState(next);
    try {
      browserStorage()?.setItem('aurora.digi.tab', next);
    } catch {
      // la scelta vale comunque finche' la pagina e' aperta
    }
  };
  const pop = useRef<HTMLDivElement>(null);
  const body = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!picking) return;
    const out = (e: PointerEvent) => {
      if (!pop.current?.contains(e.target as Node)) setPicking(false);
    };
    document.addEventListener('pointerdown', out);
    return () => document.removeEventListener('pointerdown', out);
  }, [picking]);
  const widthOf = (c: LogCol) => widths[c] ?? DEFAULT_EM[c];
  const storeWidths = (next: Partial<Record<LogCol, number>>) => {
    try {
      browserStorage()?.setItem(WIDTHS_KEY, JSON.stringify(next));
    } catch {
      // la scelta vale comunque finche' la pagina e' aperta
    }
  };
  /** Il corpo del carattere della tabella in px: le larghezze sono in em e seguono la dimensione scelta nelle Impostazioni. */
  const emPx = () => parseFloat(getComputedStyle(body.current?.querySelector('table') ?? document.body).fontSize) || 12;
  /** Trascinando il bordo destro dell'intestazione la colonna si allarga o si stringe, come in Excel. */
  const startResize = (e: React.PointerEvent, c: LogCol) => {
    e.preventDefault();
    e.stopPropagation();
    const startX = e.clientX;
    const start = widthOf(c);
    const em = emPx();
    let last = start;
    const move = (m: PointerEvent) => {
      last = Math.min(MAX_EM, Math.max(MIN_EM, start + (m.clientX - startX) / em));
      setWidths((w) => ({ ...w, [c]: last }));
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      setWidths((w) => {
        const next = { ...w, [c]: last };
        storeWidths(next);
        return next;
      });
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  };
  /** Doppio clic sul bordo: la colonna prende la larghezza del suo contenuto piu' lungo. */
  const autoFit = (c: LogCol) => {
    const table = body.current?.querySelector('table');
    if (!table) return;
    const i = cols.indexOf(c);
    let max = 0;
    table.querySelectorAll(`tr > :nth-child(${i + 1})`).forEach((el) => {
      if (el.closest('tr.digi-log-cur')) return;
      max = Math.max(max, (el as HTMLElement).scrollWidth);
    });
    const em = Math.min(MAX_EM, Math.max(MIN_EM, max / emPx() + 0.3));
    setWidths((w) => {
      const next = { ...w, [c]: em };
      storeWidths(next);
      return next;
    });
  };
  const autoFitAll = () => {
    const next: Partial<Record<LogCol, number>> = {};
    const table = body.current?.querySelector('table');
    if (table) {
      cols.forEach((c, i) => {
        let max = 0;
        table.querySelectorAll(`tr > :nth-child(${i + 1})`).forEach((el) => {
          if (el.closest('tr.digi-log-cur')) return;
          max = Math.max(max, (el as HTMLElement).scrollWidth);
        });
        next[c] = Math.min(MAX_EM, Math.max(MIN_EM, Math.max(max / emPx() + 0.3, DEFAULT_EM[c] * 0.6)));
      });
    }
    setWidths(next);
    storeWidths(next);
  };
  const resetWidths = () => {
    setWidths({});
    storeWidths({});
  };
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
    const all = (log.data?.qsos ?? []).map((q, i) => ({ q, nr: total - i }));
    const f = filter.trim().toUpperCase();
    return (f === '' ? all : all.filter((x) => x.q.call.includes(f))).slice(0, 300);
  }, [log.data, filter, total]);
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
      case 'mode': return modeName;
      case 'band': return '';
      case 'freq': return khz(dialHz > 0 ? dialHz + Math.round(audioHz) : 0);
      case 'date': return nowDate;
      case 'utc': return nowTime;
      case 'call': return input(qso.call, 16, (v) => onQso({ call: v }), true);
      case 'name': return input(qso.name, 30, (v) => onQso({ name: v }));
      case 'qth': return input(qso.qth, 40, (v) => onQso({ qth: v }));
      case 'country': return '';
      case 'grid': return input(qso.wwloc, 8, (v) => onQso({ wwloc: v }), true);
      case 'rstS': return input(rsts, 4, (v) => onRst({ rsts: v }));
      case 'rstR': return input(rstr, 4, (v) => onRst({ rstr: v }));
    }
  };

  return (
    <aside className="digi-log" aria-label={t('digi.log.title')}>
      <div
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
      />
      <div className="digi-log-tabs" role="tablist">
        <button type="button" role="tab" aria-selected={tab === 'cluster'} className={tab === 'cluster' ? 'on' : ''} onClick={() => setTab('cluster')}>
          {t('digi.tab.cluster')}
        </button>
        <button type="button" role="tab" aria-selected={tab === 'log'} className={tab === 'log' ? 'on' : ''} onClick={() => setTab('log')}>
          {t('digi.log.title')}
        </button>
        {tab === 'cluster' ? (
          <span className="digi-log-tabtools">
            <button type="button" className="sstv-btn glass" title={t('digi.spot.tip')} onClick={() => setSpotOpen(true)}>
              {t('ctl.spot')}
            </button>
            <button type="button" className="digi-tool" title={t('cop.tip')} aria-label={t('cop.tip')} onClick={() => setOptionsOpen(true)}>
              ⚙
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
                  {ALL_COLS.map((c) => (
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
            <table style={{ width: `${cols.reduce((n, c) => n + widthOf(c), 0)}em` }}>
              <colgroup>
                {cols.map((c) => (
                  <col key={c} style={{ width: `${widthOf(c)}em` }} />
                ))}
              </colgroup>
              <thead>
                <tr>
                  {cols.map((c) => (
                    <th key={c}>
                      {t(COL_LABEL[c])}
                      <span className="digi-colgrip" role="separator" aria-orientation="vertical" title={t('digi.log.colGrip')} onPointerDown={(e) => startResize(e, c)} onDoubleClick={() => autoFit(c)} />
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                <tr className="digi-log-cur">
                  {cols.map((c) => (
                    <td key={c}>{current(c)}</td>
                  ))}
                </tr>
                {rows.map(({ q, nr }) => (
                  <tr key={nr} style={{ '--b': bandColor(q.band) } as React.CSSProperties}>
                    {cols.map((c) => (
                      <td key={c}>{cell(q, nr, c)}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
            {rows.length === 0 ? <p className="digi-log-empty">{t('digi.log.empty')}</p> : null}
          </div>
          <div className="digi-log-bar">
            <button type="button" className="digi-tool" title={t('digi.log.save')} aria-label={t('digi.log.save')} onClick={onSave}>
              💾
            </button>
            <button type="button" className="digi-tool digi-tool--red" title={t('digi.log.clear')} aria-label={t('digi.log.clear')} onClick={onClear}>
              ✕
            </button>
          </div>
        </>
      )}
      {optionsOpen ? <ClusterOptionsDialog options={clusterOptions} onChange={setClusterOptions} onClose={() => setOptionsOpen(false)} /> : null}
      {spotOpen ? <SpotDialog call={qso.call} dialHz={dialHz} audioHz={audioHz} modeName={modeName} onClose={() => setSpotOpen(false)} /> : null}
    </aside>
  );
}
