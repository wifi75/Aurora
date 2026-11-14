/**
 * Il numero progressivo dei contest: quanti QSO si sono salvati finora + 1. Il contatore sta nelle preferenze del browser
 * (chiave `aurora.`, copiata anche sul server), avanza quando il QSO si salva e si puo' correggere o azzerare dal log.
 */
export const SERIAL_KEY = 'aurora.digi.qsonr';

type Store = Pick<Storage, 'getItem' | 'setItem'>;

/** Il numero del prossimo QSO (almeno 1). */
export function readSerial(store: Store | null | undefined): number {
  try {
    const n = Number(store?.getItem(SERIAL_KEY) ?? '');
    return Number.isInteger(n) && n >= 1 ? n : 1;
  } catch {
    return 1;
  }
}

/** Salva il numero del prossimo QSO (i valori non validi tornano a 1). */
export function writeSerial(store: Store | null | undefined, n: number): number {
  const v = Number.isInteger(n) && n >= 1 ? Math.min(n, 99999) : 1;
  try {
    store?.setItem(SERIAL_KEY, String(v));
  } catch {
    // il numero vale comunque finche' la pagina e' aperta
  }
  return v;
}

/** Il numero come si manda nei contest: almeno tre cifre (`001`). */
export function formatSerial(n: number): string {
  return String(Math.max(1, Math.trunc(n))).padStart(3, '0');
}

/** Lo scambio da mandare: quello scritto, altrimenti il numero progressivo. */
export function exchangeSent(nrs: string | undefined, serial: number): string {
  const v = (nrs ?? '').trim();
  return v !== '' ? v : formatSerial(serial);
}

/** Le note del QSO con gli scambi del contest in coda (il log non ha campi a parte): `note NRS 001 NRR 015`. */
export function commentWithExchange(notes: string, nrs: string, nrr: string): string {
  const ex = nrr.trim() === '' ? '' : `NRS ${nrs.trim()} NRR ${nrr.trim()}`;
  return [notes.trim(), ex].filter(Boolean).join(' ');
}

/**
 * In gara il numero viene dal log del contest: l'ultimo numero mandato (nello scambio inviato del QSO piu' recente) + 1; senza QSO
 * si riparte da 1 (`001`), quindi ogni contest attivato ricomincia da capo.
 */
export function contestNextSerial(log: readonly { n: number; exch_sent: string }[]): number {
  let last: { n: number; exch_sent: string } | null = null;
  for (const q of log) if (/\d/.test(q.exch_sent) && (!last || q.n > last.n)) last = q;
  if (!last) return 1;
  const nums = last.exch_sent.match(/\d+/g) ?? [];
  // lo scambio puo' avere anche il rapporto davanti (`599 015`): il numero e' l'ultimo gruppo di cifre
  const n = Number(nums[nums.length - 1]);
  return Number.isInteger(n) && n >= 0 ? n + 1 : 1;
}
