/** La ricerca nelle Impostazioni: i testi delle opzioni si leggono dalle schede stesse, nella lingua in uso, e si cercano senza badare a maiuscole e accenti. */

export interface SearchEntry {
  /** La scheda in cui si trova. */
  tab: string;
  /** Il testo dell'opzione, come si legge a schermo. */
  text: string;
}

/** Minuscolo e senza accenti: «Già lavorati» si trova scrivendo «gia lav». */
export function fold(text: string): string {
  return text
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase();
}

/** Gli elementi che dicono a parole cosa fa un'opzione. */
const SELECTOR = 'label, legend, h2, h3, h4, summary, th[scope="row"], [data-opt], .sx-group-title, .group-title, button[aria-label], input[aria-label], select[aria-label]';

/** Il testo breve di un elemento: da `aria-label` per i campi, altrimenti il suo testo. */
function textOf(el: Element): string {
  const aria = el.getAttribute('aria-label');
  const raw = (el.matches('input, select, button') ? aria : (el.textContent ?? '')) ?? '';
  return raw.replace(/\s+/g, ' ').trim();
}

/** Raccoglie i testi delle opzioni dentro una scheda già disegnata (senza doppioni e senza testi troppo lunghi o corti). */
export function collectEntries(tab: string, root: ParentNode): SearchEntry[] {
  const seen = new Set<string>();
  const out: SearchEntry[] = [];
  for (const el of Array.from(root.querySelectorAll(SELECTOR))) {
    const text = textOf(el);
    if (text.length < 3 || text.length > 120) continue;
    const key = fold(text);
    if (seen.has(key)) continue;
    seen.add(key);
    out.push({ tab, text });
  }
  return out;
}

/** Le voci in cui compaiono tutte le parole scritte (dalle piu' vicine all'inizio del testo). */
export function matchEntries(entries: readonly SearchEntry[], query: string, limit = 40): SearchEntry[] {
  const words = fold(query).split(/\s+/).filter(Boolean);
  if (words.length === 0) return [];
  const scored: { entry: SearchEntry; score: number }[] = [];
  for (const entry of entries) {
    const hay = fold(entry.text);
    if (!words.every((w) => hay.includes(w))) continue;
    const first = hay.indexOf(words[0] ?? '');
    scored.push({ entry, score: first + hay.length / 100 });
  }
  scored.sort((a, b) => a.score - b.score);
  return scored.slice(0, limit).map((s) => s.entry);
}
