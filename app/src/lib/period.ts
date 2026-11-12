/** MSK144: il periodo T/R in uso (5, 10, 15 o 30 s), come lo riporta lo stato del motore (`tr_period`). */
let mskPeriodSecs = 15;

/** Ricorda il periodo T/R dallo stato del motore: in MSK144 la durata non si legge dal nome del modo. */
export function noteTrPeriod(mode: string | undefined, trPeriod: number | null | undefined): void {
  if ((mode ?? '').toUpperCase() === 'MSK144' && (trPeriod === 5 || trPeriod === 10 || trPeriod === 15 || trPeriod === 30)) mskPeriodSecs = trPeriod;
}

/** I periodi T/R di FST4 (FST4W solo da 120 s in su), come in WSJT-X. */
const FST4_PERIODS = [15, 30, 60, 120, 300, 900, 1800] as const;

/** FST4 e FST4W con il periodo, come li riporta il motore («FST4-60», «FST4W-120»): il modo e il periodo; `null` per gli altri modi. */
export function splitFst4(mode: string): { base: 'FST4' | 'FST4W'; period: number | null } | null {
  const m = /^(FST4W?)(?:-(\d+))?$/.exec(mode.trim().toUpperCase());
  if (!m) return null;
  const base = m[1] as 'FST4' | 'FST4W';
  const n = m[2] === undefined ? null : Number(m[2]);
  const ok = n !== null && (FST4_PERIODS as readonly number[]).includes(n) && (base === 'FST4' || n >= 120);
  return { base, period: ok ? n : null };
}

/** Il nome del modo senza il periodo di FST4/FST4W («FST4-60» → «FST4»), per confrontarlo con le tabelle che usano il nome nudo. */
export function modeBase(mode: string): string {
  return splitFst4(mode)?.base ?? mode;
}

/**
 * Durata di un periodo in secondi: FT8 15, FT4 7,5, JT65 e JT9 60 (un minuto intero: si decodifica a fine minuto), MSK144 quello scelto,
 * FST4 e FST4W quello scritto nel nome del modo (senza: 60 e 120, i valori di partenza di WSJT-X).
 */
export function periodSecs(mode: string): number {
  const m = mode.toUpperCase();
  const fst4 = splitFst4(m);
  if (fst4) return fst4.period ?? (fst4.base === 'FST4W' ? 120 : 60);
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
