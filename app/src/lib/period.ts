/** Durata di un periodo in secondi: FT8 15, FT4 7,5, JT65 e JT9 60 (un minuto intero: si decodifica a fine minuto). */
export function periodSecs(mode: string): number {
  const m = mode.toUpperCase();
  if (m === 'FT4') return 7.5;
  if (m.startsWith('JT')) return 60;
  return 15;
}

/**
 * Dove siamo nel periodo, agganciati all'orologio come i periodi del motore (FT8: :00, :15, :30, :45).
 * `seconds` sono i secondi interi trascorsi, come l'indicatore "14/15" del motore (da 0 a durata - 1);
 * `fraction` va da 0 a 1 e fa scorrere la barra senza scatti.
 */
export function periodPosition(nowMs: number, mode: string): { seconds: number; fraction: number; length: number } {
  const length = periodSecs(mode);
  const elapsed = (((nowMs / 1000) % length) + length) % length;
  return { seconds: Math.floor(elapsed), fraction: elapsed / length, length };
}
