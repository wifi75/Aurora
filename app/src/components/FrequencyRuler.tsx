import { decodeMarkers, rulerTicks } from '../lib/ruler';
import type { DecodedMessage } from '../api/schemas';

interface Props {
  fMinHz: number;
  fMaxHz: number;
  decodes: DecodedMessage[];
}

/** Righello delle frequenze audio sotto la cascata, con un segnaposto per ogni decode dell'ultimo ciclo. */
export function FrequencyRuler({ fMinHz, fMaxHz, decodes }: Props) {
  const ticks = rulerTicks(fMinHz, fMaxHz);
  const markers = decodeMarkers(decodes, fMaxHz, fMinHz);
  return (
    <div className="ruler" aria-hidden="true">
      <div className="ruler-markers">
        {markers.map((m) => (
          <span key={m.id} className={`marker marker--${m.kind}`} style={{ left: `${m.left}%` }} />
        ))}
      </div>
      <div className="ruler-ticks">
        {ticks.map((t) => (
          <span key={t.hz} className="tick" style={{ left: `${t.left}%` }}>
            {t.hz}
          </span>
        ))}
      </div>
    </div>
  );
}
