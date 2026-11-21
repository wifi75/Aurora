/** I modi con cui si raggruppano i contest nelle linguette dell'elenco (senza «Tutti», che li mostra tutti). */
export const CONTEST_MODE_TABS = ['ssb', 'cw', 'rtty', 'psk', 'digi', 'other'] as const;
export type ContestModeTab = (typeof CONTEST_MODE_TABS)[number];

/** Una parola delle note (minuscola, solo lettere e cifre) -> il modo che nomina, o `null`. */
function modeOfWord(w: string): ContestModeTab | null {
  if (/^(ssb|phone|fonia|usb|lsb|sideband)$/.test(w)) return 'ssb';
  if (w === 'cw') return 'cw';
  if (w === 'rtty') return 'rtty';
  if (/^(b|q)?psk\d*$/.test(w)) return 'psk';
  if (/^(ft8|ft4|fst4w?|jt65|jt9|jt4|q65|msk144|wspr|digital|digitale|digitali|digi|digimodes?)$/.test(w)) return 'digi';
  return null;
}

function scan(text: string): Set<ContestModeTab> {
  const found = new Set<ContestModeTab>();
  // i separatori («&», «@» refuso di «&», «-», «/», «,», «e», «and», «+») non servono: si leggono le parole una a una
  for (const w of text.toLowerCase().match(/[a-z0-9]+/g) ?? []) {
    const m = modeOfWord(w);
    if (m) found.add(m);
  }
  return found;
}

/**
 * I modi di un contest, ricavati dal testo delle note (se non nominano nessun modo, dal nome). Un contest con piu' modi li ha tutti;
 * se nessun modo e' riconosciuto vale `['other']`. L'ordine e' quello di `CONTEST_MODE_TABS`.
 */
export function contestModes(notes: string, name = ''): ContestModeTab[] {
  let found = scan(notes);
  if (found.size === 0) found = scan(name);
  if (found.size === 0) return ['other'];
  return CONTEST_MODE_TABS.filter((m) => found.has(m));
}
