/**
 * I CQ «speciali» di FT8 e FT4 (richiesta #115): «CQ POTA IU3ABC IO91», «CQ SOTA IU3ABC/P», «CQ WWFF <IU3ABC/P> IO91» e simili.
 * Solo cinque etichette: POTA (parchi), SOTA (vette), IOTA (isole), WWFF (flora e fauna) e QRP; «CQ DX», «CQ NA», «CQ TEST» non contano.
 */

export const SPECIAL_TAGS = ['POTA', 'SOTA', 'IOTA', 'WWFF', 'QRP'] as const;
export type SpecialTag = (typeof SPECIAL_TAGS)[number];

export interface SpecialCq {
  tag: SpecialTag;
  /** Il nominativo com'e' nel messaggio, senza le parentesi angolari (IU3ABC/P). */
  call: string;
  /** Il nominativo senza prefissi e suffissi con la barra (IU3ABC): serve a trovarlo negli spot, dove a volte manca il /P. */
  base: string;
}

const GRID = /^[A-R]{2}\d{2}([A-X]{2})?$/;
/** Lettere e cifre, con pezzi separati da una barra (PJ4/IU3ABC, IU3ABC/P), mai una barra in testa o in coda. */
const CALL = /^[A-Z0-9]+(\/[A-Z0-9]+)*$/;

/** Il pezzo piu' lungo di un nominativo con la barra (PJ4/IU3ABC/P → IU3ABC); senza barra, il nominativo stesso. */
export function baseCall(call: string): string {
  const parts = call.toUpperCase().split('/').filter((p) => p !== '');
  return parts.reduce((best, p) => (p.length > best.length ? p : best), parts[0] ?? '');
}

function isCall(word: string): boolean {
  if (word.length < 3 || word.length > 20 || !/\d/.test(word) || !/[A-Z]/.test(word) || GRID.test(word)) return false;
  return CALL.test(word);
}

/** Il CQ speciale del messaggio, o `null`: «CQ <etichetta> <nominativo> [locatore]», e nient'altro dopo. */
export function parseSpecialCq(text: string): SpecialCq | null {
  const words = text.trim().toUpperCase().split(/\s+/);
  if (words.length < 3 || words.length > 4 || words[0] !== 'CQ') return null;
  const tag = SPECIAL_TAGS.find((t) => t === words[1]);
  if (!tag) return null;
  const call = (words[2] ?? '').replace(/^<|>$/g, '');
  if (!isCall(call)) return null;
  const grid = words[3];
  if (grid !== undefined && !GRID.test(grid)) return null;
  return { tag, call, base: baseCall(call) };
}
