import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useRef, useState } from 'react';
import { browserStorage } from '../../lib/theme';
import { ClusterPanel } from '../ClusterPanel';
import { SpotDialog } from './SpotDialog';
import { getLogQsos, type ClusterSpot, type LogQso } from '../../api/client';
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

/** Frequenza in kHz con la virgola, come nel log (7042,988). */
const khz = (hz: number) => (hz > 0 ? (hz / 1000).toFixed(3).replace(/(\.\d)0+$/, '$1').replace('.', ',') : '');
/** `2023-03-19` -> `19/03/2023`. */
const dmy = (d: string) => (/^\d{4}-\d{2}-\d{2}$/.test(d) ? `${d.slice(8, 10)}/${d.slice(5, 7)}/${d.slice(0, 4)}` : d);

const COL_LABEL = {
  num: 'digi.log.num', mode: 'digi.log.mode', band: 'digi.log.band', freq: 'digi.log.freq', date: 'digi.log.date', utc: 'digi.log.utc',
  call: 'digi.log.call', name: 'digi.log.name', qth: 'digi.log.qth', country: 'digi.log.country', grid: 'digi.log.grid', rstS: 'digi.log.rsts', rstR: 'digi.log.rstr',
} as const;

function cell(q: LogQso, c: LogCol): React.ReactNode {
  switch (c) {
    case 'num': return q.id;
    case 'mode': return q.mode;
    case 'band': return q.band;
    case 'freq': return khz(q.freq_hz);
    case 'date': return dmy(q.date);
    case 'utc': return q.time.length === 5 ? `${q.time}:00` : q.time;
    case 'call': return <b>{q.call}</b>;
    case 'name': return q.name ?? '';
    case 'qth': return q.qth ?? '';
    case 'country': return q.country;
    case 'grid': return q.grid;
    case 'rstS': return q.rst_sent;
    case 'rstR': return q.rst_rcvd;
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
  const [spotOpen, setSpotOpen] = useState(false);
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
  const log = useQuery({ queryKey: ['log-qsos'], queryFn: getLogQsos, retry: false, refetchInterval: 20_000 });
  // dal log, il piu' recente in testa (l'elenco arriva gia' cosi'): i primi 300
  const rows = useMemo(() => {
    const all = log.data?.qsos ?? [];
    const f = filter.trim().toUpperCase();
    return (f === '' ? all : all.filter((q) => q.call.includes(f))).slice(0, 300);
  }, [log.data, filter]);
  const nextNum = (log.data?.total ?? 0) + 1;
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
            <ClusterPanel onSpot={onSpot} />
          </div>
        </>
      ) : (
        <>
          <div className="digi-log-table digi-log-table--completa" ref={body}>
            <table>
              <thead>
                <tr>
                  {cols.map((c) => (
                    <th key={c}>{t(COL_LABEL[c])}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                <tr className="digi-log-cur">
                  {cols.map((c) => (
                    <td key={c}>{current(c)}</td>
                  ))}
                </tr>
                {rows.map((q) => (
                  <tr key={q.id}>
                    {cols.map((c) => (
                      <td key={c}>{cell(q, c)}</td>
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
      {spotOpen ? <SpotDialog call={qso.call} dialHz={dialHz} audioHz={audioHz} modeName={modeName} onClose={() => setSpotOpen(false)} /> : null}
    </aside>
  );
}
