import { decodeMarkers, rulerTicks } from '../lib/ruler';
import type { DecodedMessage } from '../api/schemas';

interface Props {
  fMinHz: number;
  fMaxHz: number;
  decodes: DecodedMessage[];
  /** Frequenze audio di ricezione e trasmissione: si disegnano come due parentesi (Tx arancione sopra, Rx verde sotto). */
  rxHz?: number | null;
  txHz?: number | null;
  /** Larghezza del segnale (Hz): la lunghezza delle parentesi. */
  bandwidthHz?: number;
  /** Dove comincia il segnale rispetto alla frequenza (Hz, MSK144 -1000: centrato). */
  startOffsetHz?: number;
}

/** Percentuale della larghezza per una frequenza, tenuta dentro il righello. */
function at(hz: number, fMinHz: number, fMaxHz: number): number {
  return Math.min(100, Math.max(0, ((hz - fMinHz) / (fMaxHz - fMinHz)) * 100));
}

/**
 * Righello stretto delle frequenze audio: una tacca ogni 100 Hz (piu' lunga e con il numero ogni 500), le parentesi di Tx e Rx e,
 * lungo il bordo basso, un segno per ogni decodifica dell'ultimo periodo.
 */
export function FrequencyRuler({ fMinHz, fMaxHz, decodes, rxHz, txHz, bandwidthHz = 50, startOffsetHz = 0 }: Props) {
  const ticks = rulerTicks(fMinHz, fMaxHz, 100);
  const markers = decodeMarkers(decodes, fMaxHz, fMinHz);
  const span = fMaxHz - fMinHz;
  const bracket = (hz: number | null | undefined, kind: 'tx' | 'rx') => {
    if (hz === null || hz === undefined || !(span > 0)) return null;
    const left = at(hz + startOffsetHz, fMinHz, fMaxHz);
    const width = Math.max(0.6, (Math.max(bandwidthHz, 20) / span) * 100);
    return <span className={`ruler-bracket ruler-bracket--${kind}`} style={{ left: `${left}%`, width: `${width}%` }} />;
  };
  return (
    <div className="ruler" aria-hidden="true">
      <div className="ruler-strip">
        {ticks.map((tick) => {
          const major = tick.hz % 500 === 0;
          return (
            <span key={tick.hz} className={`ruler-tick${major ? ' ruler-tick--major' : ''}`} style={{ left: `${tick.left}%` }}>
              {major ? <em>{tick.hz}</em> : null}
            </span>
          );
        })}
        {bracket(txHz, 'tx')}
        {bracket(rxHz, 'rx')}
        {markers.map((m) => (
          <span key={m.id} className={`marker marker--${m.kind}`} style={{ left: `${m.left}%` }} />
        ))}
      </div>
    </div>
  );
}
