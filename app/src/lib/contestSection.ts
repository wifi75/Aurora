/**
 * Il terzo elemento dello scambio dei contest che lo chiedono (sezione, club, stato, provincia): la casella della riga di inserimento
 * si vede solo quando serve (`exch_atteso` dal core, con le stesse regole del punteggio), il valore si compone con il numero ricevuto
 * (`003 ACC`) e la propria sezione si ricorda per contest nelle preferenze del browser (chiave `aurora.`, copiata anche sul server).
 */
export const SECTION_KEY = 'aurora.digi.contestsect.';

/** Lunghezza massima della sezione, dello stato, del club o del valore proprio (il SARTG New Year chiede nome e augurio per esteso). */
export const EXTRA_MAX = 20;

type Store = Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>;

/** Il testo di una sezione: maiuscolo, una sola riga, al massimo `EXTRA_MAX` caratteri. */
export function cleanExtra(v: string | undefined): string {
  return (v ?? '').toUpperCase().replace(/\s+/g, ' ').trim().slice(0, EXTRA_MAX);
}

/** La propria sezione / stato / club ricordata per il contest (vuota se non scritta). */
export function readSection(store: Store | null | undefined, contestId: string | null | undefined): string {
  if (!contestId) return '';
  try {
    return cleanExtra(store?.getItem(`${SECTION_KEY}${contestId}`) ?? '');
  } catch {
    return '';
  }
}

/** Ricorda la propria sezione per il contest (vuota = tolta). */
export function writeSection(store: Store | null | undefined, contestId: string | null | undefined, value: string): void {
  if (!contestId) return;
  try {
    const v = cleanExtra(value);
    if (v === '') store?.removeItem(`${SECTION_KEY}${contestId}`);
    else store?.setItem(`${SECTION_KEY}${contestId}`, v);
  } catch {
    // vale comunque finche' la pagina e' aperta
  }
}

/** Lo scambio ricevuto come lo si registra: numero e terzo elemento separati da uno spazio (`003 ACC`). */
export function composeReceived(nrr: string | undefined, extra: string | undefined): string {
  return `${(nrr ?? '').trim()} ${(extra ?? '').trim()}`.trim();
}

/** Lo scambio inviato: il numero e, se alla propria stazione tocca mandarla (`applies`) e la si e' scritta, la propria sezione (`001 ACC`). */
export function composeSent(nrs: string, section: string, applies: boolean): string {
  return applies ? `${nrs.trim()} ${cleanExtra(section)}`.trim() : nrs.trim();
}

/** Come si chiama la casella: UBA «Club (sezione UBA)», ARRL «Stato / provincia», altrimenti il nome generico. */
export function extraLabelKey(contestName: string | null | undefined): 'ssbc.exLabel.uba' | 'ssbc.exLabel.arrl' | 'ssbc.exLabel.generic' {
  const n = (contestName ?? '').toUpperCase();
  if (/\bUBA\b/.test(n)) return 'ssbc.exLabel.uba';
  if (/\bARRL\b/.test(n)) return 'ssbc.exLabel.arrl';
  return 'ssbc.exLabel.generic';
}

/** Le regole del punteggio hanno un moltiplicatore `exch`? (per mostrare dove scrivere la propria sezione nelle impostazioni). */
export function hasExchMultiplier(score: string | undefined): boolean {
  return /^\s*mult\s*=(.*;)?\s*exch\s*(\/|;|$)/im.test(score ?? '');
}
