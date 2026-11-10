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

export type LogCol = 'mode' | 'band' | 'freq' | 'date' | 'utc' | 'call' | 'country' | 'grid' | 'rst';
const ALL_COLS: readonly LogCol[] = ['mode', 'band', 'freq', 'date', 'utc', 'call', 'country', 'grid', 'rst'];
const DEFAULT_COLS: readonly LogCol[] = ['mode', 'freq', 'date', 'utc', 'call', 'country', 'rst'];
const COLS_KEY = 'aurora.digi.logcols';

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

const mhz = (hz: number) => (hz > 0 ? (hz / 1_000_000).toFixed(6).replace(/0+$/, '').replace(/\.$/, '') : '');

const COL_LABEL = {
  mode: 'digi.log.mode', band: 'digi.log.band', freq: 'digi.log.freq', date: 'digi.log.date', utc: 'digi.log.utc',
  call: 'digi.log.call', country: 'digi.log.country', grid: 'digi.log.grid', rst: 'digi.log.rst',
} as const;

function cell(q: LogQso, c: LogCol): React.ReactNode {
  switch (c) {
    case 'mode': return q.mode;
    case 'band': return q.band;
    case 'freq': return mhz(q.freq_hz);
    case 'date': return q.date;
    case 'utc': return q.time.slice(0, 5);
    case 'call': return <b>{q.call}</b>;
    case 'country': return q.country;
    case 'grid': return q.grid;
    case 'rst': return `${q.rst_sent}/${q.rst_rcvd}`;
  }
}

/** Il log di lato: la riga del QSO in corso (si scrive qui) e sotto i QSO fatti, piu' recenti per primi. */
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
  const rows = useMemo(() => {
    const all = [...(log.data?.qsos ?? [])].reverse();
    const f = filter.trim().toUpperCase();
    return (f === '' ? all : all.filter((q) => q.call.includes(f))).slice(0, 80);
  }, [log.data, filter]);
  const now = new Date();

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
      <div className="digi-log-qsohead">
        <strong>{t('digi.log.current')}</strong>
        <button type="button" className="digi-tool" title={t('digi.log.save')} aria-label={t('digi.log.save')} onClick={onSave}>
          💾
        </button>
        <button type="button" className="digi-tool digi-tool--red" title={t('digi.log.clear')} aria-label={t('digi.log.clear')} onClick={onClear}>
          ✕
        </button>
      </div>
      <div className="digi-log-current" role="group" aria-label={t('digi.log.current')}>
        <label>
          <span>{t('digi.log.call')}</span>
          <input value={qso.call} maxLength={16} onChange={(e) => onQso({ call: e.target.value.toUpperCase() })} className="digi-call" />
        </label>
        <label>
          <span>{t('digi.log.name')}</span>
          <input value={qso.name} maxLength={30} onChange={(e) => onQso({ name: e.target.value })} />
        </label>
        <label>
          <span>{t('digi.log.qth')}</span>
          <input value={qso.qth} maxLength={40} onChange={(e) => onQso({ qth: e.target.value })} />
        </label>
        <label className="digi-rst">
          <span>{t('digi.f.rsts')}</span>
          <input value={rsts} maxLength={4} onChange={(e) => onRst({ rsts: e.target.value })} />
        </label>
        <label className="digi-rst">
          <span>{t('digi.f.rstr')}</span>
          <input value={rstr} maxLength={4} onChange={(e) => onRst({ rstr: e.target.value })} />
        </label>
        <label className="digi-wide">
          <span>{t('digi.f.notes')}</span>
          <input value={qso.notes} maxLength={80} onChange={(e) => onQso({ notes: e.target.value })} />
        </label>
        <p className="digi-log-now">
          {modeName} · {mhz(dialHz) || '—'} MHz · {now.toISOString().slice(0, 10)} {now.toISOString().slice(11, 16)}z
        </p>
      </div>

      {tab === 'cluster' ? (
        <div className="digi-log-cluster">
          <ClusterPanel onSpot={onSpot} />
        </div>
      ) : null}
      <div className="digi-log-table" hidden={tab !== 'log'}>
        <table>
          <thead>
            <tr>
              {cols.map((c) => (
                <th key={c}>{t(COL_LABEL[c])}</th>
              ))}
            </tr>
          </thead>
          <tbody>
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
      {spotOpen ? <SpotDialog call={qso.call} dialHz={dialHz} audioHz={audioHz} modeName={modeName} onClose={() => setSpotOpen(false)} /> : null}
    </aside>
  );
}
