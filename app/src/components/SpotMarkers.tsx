import { useQuery } from '@tanstack/react-query';
import { getClusterSpots } from '../api/client';
import { useT } from '../i18n';
import { spotsOnWaterfall } from '../lib/cluster';
import { formatKhz } from '../lib/format';
import { leftPercent } from '../lib/markers';

interface Props {
  fMinHz: number;
  fMaxHz: number;
  /** Frequenza della radio (Hz): lo spot cade sulla cascata alla sua distanza da questa. */
  dialHz: number | undefined;
  /** Un clic su uno spot: porta la frequenza di ricezione li' (solo ricezione, non trasmette). */
  onPick(hz: number): void;
}

/** Gli spot del DX Cluster sulla cascata, in basso: tacca e nominativo alla loro frequenza audio. Usa gli stessi spot della finestra Cluster. */
export function SpotMarkers({ fMinHz, fMaxHz, dialHz, onPick }: Props) {
  const t = useT();
  const cluster = useQuery({ queryKey: ['dx-cluster'], queryFn: () => getClusterSpots(), retry: false, refetchInterval: 4000 });
  const config = cluster.data?.config;
  if (!config?.enabled || !dialHz) return null;
  const spots = spotsOnWaterfall(cluster.data?.spots ?? [], dialHz, fMinHz, fMaxHz);
  if (spots.length === 0) return null;

  return (
    <div className="wf-spots" aria-hidden={false}>
      {spots.map((s) => (
        <button
          key={s.call}
          type="button"
          className={`wf-spot${s.row === null ? ' wf-spot--tick' : ''}`}
          style={{ left: `${leftPercent(s.hz, fMinHz, fMaxHz)}%`, bottom: `${(s.row ?? 0) * 1.05}rem` }}
          title={t('wf.spot', { call: s.call, spotter: s.spotter, freq: formatKhz(s.freq_khz) })}
          aria-label={t('wf.spot', { call: s.call, spotter: s.spotter, freq: formatKhz(s.freq_khz) })}
          onPointerDown={(e) => e.stopPropagation()}
          onClick={() => onPick(s.hz)}
        >
          {s.row === null ? null : <span className="wf-spot-call">{s.call}</span>}
        </button>
      ))}
    </div>
  );
}
