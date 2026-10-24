/** Il cursore "Pwr" del motore e' un'attenuazione da 0 a 45 dB; qui si mostra come livello (100 % = nessuna attenuazione). */
export const MAX_ATTENUATION_DB = 45;

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

export function levelPercent(attenuationDb: number): number {
  return Math.round((1 - clamp(attenuationDb, 0, MAX_ATTENUATION_DB) / MAX_ATTENUATION_DB) * 100);
}

/** Attenuazione in dB (decimi) per un livello in percentuale. */
export function attenuationFromPercent(percent: number): number {
  const db = (1 - clamp(percent, 0, 100) / 100) * MAX_ATTENUATION_DB;
  return Math.round(db * 10) / 10;
}
