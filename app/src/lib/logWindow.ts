/**
 * La finestra di tempo dei segni del log fuori gara, scelta dalle macro: `<IGNOREBEFORE:aaaa-mm-gg hh:mm>` (i QSO di prima non contano,
 * `<IGNOREBEFORE:ERASE>` toglie il limite) e `<PERIOD:n>` (solo gli ultimi n minuti, 0 = spento). Stanno nelle preferenze (chiavi
 * `aurora.`, copiate anche sul server).
 */
export const IGNORE_BEFORE_KEY = 'aurora.digi.ignoreBefore';
export const PERIOD_KEY = 'aurora.digi.period';

type Store = Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>;

/** L'argomento di `<IGNOREBEFORE:...>`: ms UTC, `'erase'`, o `null` se non si capisce. */
export function parseIgnoreBefore(arg: string): number | 'erase' | null {
  const s = arg.trim();
  if (s.toUpperCase() === 'ERASE') return 'erase';
  const m = /^(\d{4})-(\d{2})-(\d{2})(?:[ T](\d{2}):(\d{2}))?$/.exec(s);
  if (!m) return null;
  const ms = Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3]), Number(m[4] ?? 0), Number(m[5] ?? 0));
  return Number.isFinite(ms) ? ms : null;
}

const readNum = (store: Store | null | undefined, key: string): number => {
  try {
    const n = Number(store?.getItem(key) ?? '');
    return Number.isFinite(n) && n > 0 ? n : 0;
  } catch {
    return 0;
  }
};

export function writeIgnoreBefore(store: Store | null | undefined, v: number | 'erase'): void {
  try {
    if (v === 'erase') store?.removeItem(IGNORE_BEFORE_KEY);
    else store?.setItem(IGNORE_BEFORE_KEY, String(v));
  } catch {
    // vale finche' la pagina e' aperta
  }
}

export function writePeriod(store: Store | null | undefined, minutes: number): void {
  try {
    const m = Math.max(0, Math.min(Math.trunc(minutes), 525_600));
    if (m === 0) store?.removeItem(PERIOD_KEY);
    else store?.setItem(PERIOD_KEY, String(m));
  } catch {
    // vale finche' la pagina e' aperta
  }
}

/** Da quando contano i QSO (ms UTC), o `null` se contano tutti. */
export function logWindowStart(store: Store | null | undefined, nowMs: number): number | null {
  const before = readNum(store, IGNORE_BEFORE_KEY);
  const period = readNum(store, PERIOD_KEY);
  const from = Math.max(before, period > 0 ? nowMs - period * 60_000 : 0);
  return from > 0 ? from : null;
}

/** L'istante di un QSO del log (`aaaa-mm-gg` e `hh:mm[:ss]` UTC), o `NaN`. */
export function qsoTimeMs(q: { date: string; time: string }): number {
  const d = /^(\d{4})-?(\d{2})-?(\d{2})$/.exec(q.date.trim());
  const t = /^(\d{2}):?(\d{2})(?::?(\d{2}))?/.exec(q.time.trim());
  if (!d) return Number.NaN;
  return Date.UTC(Number(d[1]), Number(d[2]) - 1, Number(d[3]), Number(t?.[1] ?? 0), Number(t?.[2] ?? 0), Number(t?.[3] ?? 0));
}
