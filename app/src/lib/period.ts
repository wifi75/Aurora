/** Durata di un periodo in secondi: FT8 15, FT4 7,5. */
export function periodSecs(mode: string): number {
  return mode === 'FT4' ? 7.5 : 15;
}

/**
 * Dove siamo nel periodo, agganciati all'orologio come i periodi di JTDX (FT8: :00, :15, :30, :45).
 * `seconds` sono i secondi interi trascorsi, come l'indicatore "14/15" di JTDX (da 0 a durata - 1);
 * `fraction` va da 0 a 1 e fa scorrere la barra senza scatti.
 */
export function periodPosition(nowMs: number, mode: string): { seconds: number; fraction: number; length: number } {
  const length = periodSecs(mode);
  const elapsed = (((nowMs / 1000) % length) + length) % length;
  return { seconds: Math.floor(elapsed), fraction: elapsed / length, length };
}
