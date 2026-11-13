import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { getHealthResources, putEngineSettings, type HealthResources } from '../api/client';
import { useT, type Key } from '../i18n';
import { useHealthIssues } from '../lib/healthIssues';
import { decodeSlowOften, healthRows, overall, type HealthRows, type Level } from '../lib/healthLight';

/** Per quanto la spia pulsa dopo un cambio verso ambra o rosso. */
const PULSE_MS = 4000;

const fmt1 = (n: number) => new Intl.NumberFormat(document.documentElement.lang || undefined, { minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(n);

function stateKey(l: Level): Key {
  return `hl.state.${l}` as Key;
}

/** Il grafico degli ultimi 10 minuti: CPU % e decodifica in % del tempo disponibile, con la soglia del 70 % tratteggiata. */
function HealthChart({ r }: { r: HealthResources | undefined }) {
  const t = useT();
  const h = r?.history ?? [];
  if (h.length < 2) return <p className="hl-muted">{t('hl.chart.empty')}</p>;
  const W = 520;
  const H = 140;
  const x = (i: number) => (i / (h.length - 1)) * W;
  const y = (pct: number) => H - (Math.min(Math.max(pct, 0), 120) / 120) * H;
  const cpu = h.map((s, i) => `${x(i).toFixed(1)},${y(s.cpu_pct).toFixed(1)}`).join(' ');
  const dec = h.map((s, i) => `${x(i).toFixed(1)},${y(s.period_ms > 0 ? (s.decode_ms / s.period_ms) * 100 : 0).toFixed(1)}`).join(' ');
  return (
    <figure className="hl-chart">
      <figcaption>{t('hl.chart')}</figcaption>
      <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" role="img" aria-label={t('hl.chart')}>
        <line x1="0" x2={W} y1={y(70)} y2={y(70)} className="hl-chart-limit" />
        <line x1="0" x2={W} y1={y(100)} y2={y(100)} className="hl-chart-grid" />
        <polyline points={cpu} className="hl-chart-cpu" />
        <polyline points={dec} className="hl-chart-dec" />
      </svg>
      <div className="hl-chart-legend">
        <span>
          <i className="hl-sw hl-sw--cpu" />
          {t('hl.chart.cpu')}
        </span>
        <span>
          <i className="hl-sw hl-sw--dec" />
          {t('hl.chart.decode')}
        </span>
        <span>
          <i className="hl-sw hl-sw--limit" />
          {t('hl.chart.limit')}
        </span>
      </div>
    </figure>
  );
}

function decodeText(r: HealthResources | undefined, t: ReturnType<typeof useT>): string {
  if (!r || r.last_decode_ms === null || !r.period_ms) return t('hl.state.none');
  return t('hl.decodeOf', { a: fmt1(r.last_decode_ms / 1000), b: fmt1(r.period_ms / 1000) });
}

function HealthDialog({ r, rows, onClose }: { r: HealthResources | undefined; rows: HealthRows; onClose: () => void }) {
  const t = useT();
  const client = useQueryClient();
  const [slowAnswer, setSlowAnswer] = useState<'none' | 'later' | 'done'>('none');
  const closeRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    closeRef.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);
  const turnOnSlowPc = async () => {
    try {
      client.setQueryData(['engine-settings'], await putEngineSettings({ SlowPcMode: 'always' }));
      setSlowAnswer('done');
    } catch {
      setSlowAnswer('none');
    }
  };
  const line = (level: Level, label: Key, value: string, todo: Key) => (
    <li className="hl-row">
      <i className={`hl-dot hl-dot--${level}`} aria-hidden="true" />
      <div>
        <b>{t(label)}</b> · {value}
        <small>{level === 'warn' || level === 'bad' ? t(todo) : t('hl.do.ok')}</small>
      </div>
    </li>
  );
  return createPortal(
    <div className="hl-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="hl-dialog" role="dialog" aria-modal="true" aria-labelledby="hl-title">
        <header>
          <h2 id="hl-title">{t('hl.title')}</h2>
          <button type="button" className="hl-x" aria-label={t('hl.close')} onClick={onClose}>
            ×
          </button>
        </header>
        <ul className="hl-rows">
          {line(rows.cpu, 'hl.cpu', r ? `${fmt1(r.cpu_pct)} %${r.mem_mb !== null ? ` · ${t('hl.mem')} ${r.mem_mb} MB` : ''}` : t('hl.state.none'), 'hl.do.cpu')}
          {line(rows.decode, 'hl.decode', `${decodeText(r, t)}${r?.decode_threads ? ` · ${t('hl.threads')} ${r.decode_threads}` : ''}`, 'hl.do.decode')}
          {line(rows.audio, 'hl.audio', t(stateKey(rows.audio)), 'hl.do.audio')}
          {line(rows.cat, 'hl.cat', t(stateKey(rows.cat)), 'hl.do.cat')}
          {line(rows.clock, 'hl.clock', t(stateKey(rows.clock)), 'hl.do.clock')}
        </ul>
        <HealthChart r={r} />
        {decodeSlowOften(r) && slowAnswer !== 'later' ? (
          <div className="hl-advice">
            {slowAnswer === 'done' ? (
              <p>{t('hl.slow.done')}</p>
            ) : (
              <>
                <p>{t('hl.slow.ask')}</p>
                <button type="button" className="hl-btn hl-btn--primary" onClick={() => void turnOnSlowPc()}>
                  {t('hl.slow.yes')}
                </button>
                <button type="button" className="hl-btn" onClick={() => setSlowAnswer('later')}>
                  {t('hl.slow.no')}
                </button>
              </>
            )}
          </div>
        ) : null}
        <footer>
          <button ref={closeRef} type="button" className="hl-btn" onClick={onClose}>
            {t('hl.close')}
          </button>
        </footer>
      </div>
    </div>,
    document.body,
  );
}

/** La spia di salute: pallino verde, ambra o rosso; passando il mouse i numeri, con un clic la finestra con i dettagli. */
export function HealthLight({ compact = false }: { compact?: boolean }) {
  const t = useT();
  const issues = useHealthIssues();
  const q = useQuery({ queryKey: ['health-resources'], queryFn: getHealthResources, retry: false, refetchInterval: 4000 });
  const rows = healthRows(q.data, issues);
  const level = overall(rows);
  const [open, setOpen] = useState(false);
  const close = useCallback(() => setOpen(false), []);
  const [hover, setHover] = useState(false);
  const [pulse, setPulse] = useState(false);
  const prev = useRef(level);
  useEffect(() => {
    if (prev.current === level) return;
    prev.current = level;
    if (level === 'ok') {
      setPulse(false);
      return;
    }
    setPulse(true);
    const id = setTimeout(() => setPulse(false), PULSE_MS);
    return () => clearTimeout(id);
  }, [level]);
  const r = q.data;
  return (
    <span className={`hl${compact ? ' hl--compact' : ''}`} onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}>
      <button
        type="button"
        className="hl-btn-dot"
        aria-label={`${t('hl.title')}: ${t(stateKey(level))}`}
        onClick={() => {
          setHover(false);
          setOpen(true);
        }}
        onFocus={() => setHover(true)}
        onBlur={() => setHover(false)}
      >
        <i className={`hl-dot hl-dot--${level}${pulse ? ' hl-dot--pulse' : ''}`} aria-hidden="true" />
      </button>
      {hover && !open ? (
        <span className="hl-tip" role="tooltip">
          <b>
            {t('hl.title')}: {t(stateKey(level))}
          </b>
          <span>
            {t('hl.cpu')}: {r ? `${fmt1(r.cpu_pct)} %` : '—'}
          </span>
          <span>
            {t('hl.mem')}: {r?.mem_mb != null ? `${r.mem_mb} MB` : '—'}
          </span>
          <span>
            {t('hl.decode')}: {decodeText(r, t)}
          </span>
          <span>
            {t('hl.threads')}: {r?.decode_threads ?? '—'}
          </span>
          <span>
            {t('hl.audio')}: {t(stateKey(rows.audio))}
          </span>
          <span>
            {t('hl.cat')}: {t(stateKey(rows.cat))}
          </span>
          <span>
            {t('hl.clock')}: {t(stateKey(rows.clock))}
          </span>
          <em>{t('hl.open')}</em>
        </span>
      ) : null}
      {open ? <HealthDialog r={r} rows={rows} onClose={close} /> : null}
    </span>
  );
}
