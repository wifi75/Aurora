/** MSK144: il periodo T/R in uso (5, 10, 15 o 30 s), come lo riporta lo stato del motore (`tr_period`). */
let mskPeriodSecs = 15;

/** Ricorda il periodo T/R dallo stato del motore: in MSK144 la durata non si legge dal nome del modo. */
export function noteTrPeriod(mode: string | undefined, trPeriod: number | null | undefined): void {
  if ((mode ?? '').toUpperCase() === 'MSK144' && (trPeriod === 5 || trPeriod === 10 || trPeriod === 15 || trPeriod === 30)) mskPeriodSecs = trPeriod;
}

/** Durata di un periodo in secondi: FT8 15, FT4 7,5, JT65 e JT9 60 (un minuto intero: si decodifica a fine minuto), MSK144 quello scelto. */
export function periodSecs(mode: string): number {
  const m = mode.toUpperCase();
  if (m === 'FT4') return 7.5;
  if (m === 'MSK144') return mskPeriodSecs;
  // WSPR-2: due minuti, dai minuti pari
  if (m === 'WSPR') return 120;
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
