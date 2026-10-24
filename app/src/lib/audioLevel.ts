/** Sotto questo valore (dBFS) l'indicatore e' vuoto. */
export const METER_FLOOR_DB = -60;

/** Ampiezza 0-1 in dBFS; il silenzio assoluto vale il minimo dell'indicatore. */
export function toDb(amplitude: number): number {
  if (!Number.isFinite(amplitude) || amplitude <= 0) return METER_FLOOR_DB;
  return Math.max(METER_FLOOR_DB, 20 * Math.log10(amplitude));
}

/** Posizione sull'indicatore, 0-100, da -60 dBFS a 0 dBFS. */
export function meterPercent(amplitude: number): number {
  return Math.round(((toDb(amplitude) - METER_FLOOR_DB) / -METER_FLOOR_DB) * 100);
}

/** Dopo quanto senza misure l'indicatore si nasconde (ms): l'ingresso audio non e' attivo. */
export const LEVEL_STALE_MS = 5_000;
