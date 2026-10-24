import type { DecodedMessage } from '../api/schemas';

export interface Tick {
  hz: number;
  /** Posizione in percentuale della larghezza (0-100). */
  left: number;
}

/** Una tacca ogni `stepHz`, a partire dal primo multiplo del passo dentro l'intervallo. */
export function rulerTicks(fMinHz: number, fMaxHz: number, stepHz = 500): Tick[] {
  if (!(fMaxHz > fMinHz) || !(stepHz > 0)) return [];
  const ticks: Tick[] = [];
  for (let hz = Math.ceil(fMinHz / stepHz) * stepHz; hz <= fMaxHz; hz += stepHz) {
    ticks.push({ hz, left: ((hz - fMinHz) / (fMaxHz - fMinHz)) * 100 });
  }
  return ticks;
}

export type MarkerKind = 'cq' | 'rare' | 'other';

export interface Marker {
  id: number;
  left: number;
  kind: MarkerKind;
}

/** Segnaposto sul righello: i decode del ciclo piu' recente, ciascuno alla sua frequenza audio. */
export function decodeMarkers(decodes: DecodedMessage[], fMaxHz: number, fMinHz = 0): Marker[] {
  if (decodes.length === 0 || !(fMaxHz > fMinHz)) return [];
  const newest = Math.max(...decodes.map((d) => d.time_ms));
  return decodes
    .filter((d) => d.time_ms === newest)
    .map((d) => ({
      id: d.id,
      left: Math.min(100, Math.max(0, ((d.freq_hz - fMinHz) / (fMaxHz - fMinHz)) * 100)),
      kind: d.tag === 'new_dxcc' || d.tag === 'new_zone' ? 'rare' : d.tag === 'cq' ? 'cq' : 'other',
    }));
}
