import type { DecodedMessage } from '../api/schemas';

/** Scarto dell'orologio in millisecondi con segno, come testo: `+120` (PC indietro) o `-80` (PC avanti). */
export function offsetLabel(offsetMs: number): { ms: number; behind: boolean } {
  return { ms: Math.round(Math.abs(offsetMs)), behind: offsetMs >= 0 };
}

/** L'orologio e' in tolleranza se lo scarto, in valore assoluto, non supera la soglia. */
export function clockInTolerance(offsetMs: number, warnMs: number): boolean {
  return Math.abs(offsetMs) <= warnMs;
}

/**
 * Il DT mediano dei messaggi appena decodificati, come il "Med" di JTDX: se l'orologio e' giusto sta vicino a zero.
 * Si guardano i messaggi dell'ultimo periodo (i piu' recenti per primi) e servono almeno tre per fidarsi.
 */
export function medianDt(decodes: readonly DecodedMessage[], windowMs = 20_000): number | null {
  const newest = decodes[0];
  if (!newest) return null;
  const values = decodes
    .filter((d) => d.id > 0 && Math.abs(newest.time_ms - d.time_ms) <= windowMs)
    .map((d) => d.delta_time)
    .filter((v) => Number.isFinite(v))
    .sort((a, b) => a - b);
  if (values.length < 3) return null;
  const mid = Math.floor(values.length / 2);
  const median = values.length % 2 === 1 ? (values[mid] ?? 0) : ((values[mid - 1] ?? 0) + (values[mid] ?? 0)) / 2;
  return Math.round(median * 100) / 100;
}
