import { useEffect, useState } from 'react';
import { formatClock } from '../lib/format';
import { periodPosition } from '../lib/period';
import { useEngineTx } from '../state/engineTx';

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** La data UTC come «2026 Nov 06». */
function utcDate(ms: number): string {
  const d = new Date(ms);
  return `${d.getUTCFullYear()} ${MONTHS[d.getUTCMonth()]} ${String(d.getUTCDate()).padStart(2, '0')}`;
}

/**
 * In cima ai Comandi: l'anello con i secondi del periodo in corso, l'ora UTC grande con la data e, a destra, il messaggio da cui parte
 * la trasmissione (CQ o Tx1-Tx5).
 */
export function PeriodClockRing({ mode, label }: { mode: string; label: string }) {
  const [now, setNow] = useState(() => Date.now());
  const selected = useEngineTx((s) => s.tx?.selected ?? null);
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 250);
    return () => window.clearInterval(timer);
  }, []);
  const { seconds, fraction, length } = periodPosition(now, mode || 'FT8');
  const radius = 21;
  const circ = 2 * Math.PI * radius;
  return (
    <div className="cmd-clock">
      <div className="cmd-ring" role="progressbar" aria-label={label} aria-valuemin={0} aria-valuemax={length} aria-valuenow={seconds}>
        <svg viewBox="0 0 50 50" aria-hidden="true">
          <circle className="cmd-ring-bg" cx="25" cy="25" r={radius} />
          <circle className="cmd-ring-fg" cx="25" cy="25" r={radius} strokeDasharray={`${circ * fraction} ${circ}`} transform="rotate(-90 25 25)" />
        </svg>
        <span className="data">{seconds}</span>
      </div>
      <div className="cmd-clock-text">
        <time className="data cmd-clock-time" dateTime={new Date(now).toISOString()}>
          {formatClock(now)}
        </time>
        <span className="data cmd-clock-date">{`${utcDate(now)} · UTC`}</span>
      </div>
      <span className="cmd-clock-next data">{selected === null ? '—' : selected === 6 ? 'CQ' : `Tx${selected}`}</span>
    </div>
  );
}
