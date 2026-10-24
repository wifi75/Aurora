import { useQuery } from '@tanstack/react-query';
import { getClock } from '../api/client';
import { useT } from '../i18n';
import { clockInTolerance, medianDt, offsetLabel } from '../lib/clock';
import { useStation } from '../state/station';

/** Nella barra di stato: lo scarto dell'orologio dai server NTP e il DT mediano dei messaggi, come il "Med" del motore. */
export function ClockPill() {
  const t = useT();
  const clock = useQuery({ queryKey: ['clock'], queryFn: getClock, retry: false, refetchInterval: 60_000 });
  const decodes = useStation((s) => s.decodes);
  const dt = medianDt(decodes);
  const last = clock.data?.last ?? null;
  const offset = last?.offset_ms ?? null;
  if (offset === null && dt === null) return null;

  const warn = clock.data?.config.warn_ms ?? 200;
  const good = offset === null ? true : clockInTolerance(offset, warn);
  const shown = offset === null ? null : offsetLabel(offset);
  return (
    <span className={`pill data ${good ? 'pill--rx' : 'pill--warn'}`} title={t('tip.clock.pill')}>
      {shown ? (
        <>
          {t('clock.pill')} {shown.behind ? '+' : '−'}
          {shown.ms} ms
        </>
      ) : (
        t('clock.pill')
      )}
      {dt !== null ? <span className="muted"> · {t('clock.dt', { dt: dt.toFixed(2) })}</span> : null}
    </span>
  );
}
