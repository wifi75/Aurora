/**
 * Una scritta come sul display di una radio, con cifre a 14 segmenti (le 7 segmenti non sanno scrivere bene le lettere: la «R» e la «D»
 * non si leggevano, scelta del proprietario). Restituisce un SVG da usare come immagine (la scritta «RADIO SPENTA» sui display a radio spenta).
 *
 * Ogni carattere sta in una cella 10×16 (piu' lo spazio in alto per gli accenti); i segmenti spenti restano appena visibili come su un display
 * vero. Lettere non previste: uno spazio.
 */

type Seg = 'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G1' | 'G2' | 'H' | 'I' | 'J' | 'K' | 'L' | 'M';

/** I segmenti: estremi in coordinate della cella (x 0..10, y 0..16). */
const SEGMENTS: Record<Seg, [number, number, number, number]> = {
  A: [1.2, 0, 8.8, 0],
  B: [10, 1.2, 10, 6.8],
  C: [10, 9.2, 10, 14.8],
  D: [1.2, 16, 8.8, 16],
  E: [0, 9.2, 0, 14.8],
  F: [0, 1.2, 0, 6.8],
  G1: [1.2, 8, 4.3, 8],
  G2: [5.7, 8, 8.8, 8],
  H: [1.4, 1.6, 4.2, 6.6],
  I: [5, 1.4, 5, 6.6],
  J: [8.6, 1.6, 5.8, 6.6],
  K: [4.2, 9.4, 1.4, 14.4],
  L: [5, 9.4, 5, 14.6],
  M: [5.8, 9.4, 8.6, 14.4],
};

const ALL = Object.keys(SEGMENTS) as Seg[];

/** Le lettere che servono alle scritte in tutte le lingue di Aurora, disegnate per leggersi bene. */
const GLYPHS: Record<string, Seg[]> = {
  A: ['A', 'B', 'C', 'E', 'F', 'G1', 'G2'],
  B: ['A', 'B', 'C', 'D', 'I', 'L', 'G2'],
  C: ['A', 'D', 'E', 'F'],
  D: ['A', 'B', 'C', 'D', 'I', 'L'],
  E: ['A', 'D', 'E', 'F', 'G1', 'G2'],
  F: ['A', 'E', 'F', 'G1', 'G2'],
  G: ['A', 'C', 'D', 'E', 'F', 'G2'],
  H: ['B', 'C', 'E', 'F', 'G1', 'G2'],
  I: ['A', 'D', 'I', 'L'],
  K: ['E', 'F', 'G1', 'J', 'M'],
  L: ['D', 'E', 'F'],
  M: ['B', 'C', 'E', 'F', 'H', 'J'],
  N: ['B', 'C', 'E', 'F', 'H', 'M'],
  O: ['A', 'B', 'C', 'D', 'E', 'F'],
  P: ['A', 'B', 'E', 'F', 'G1', 'G2'],
  R: ['A', 'B', 'E', 'F', 'G1', 'G2', 'M'],
  S: ['A', 'C', 'D', 'F', 'G1', 'G2'],
  T: ['A', 'I', 'L'],
  U: ['B', 'C', 'D', 'E', 'F'],
  V: ['E', 'F', 'K', 'J'],
  W: ['B', 'C', 'E', 'F', 'K', 'M'],
  X: ['H', 'J', 'K', 'M'],
  Y: ['H', 'J', 'L'],
  Z: ['A', 'D', 'J', 'K'],
  '0': ['A', 'B', 'C', 'D', 'E', 'F', 'J', 'K'],
  '1': ['B', 'C'],
  '-': ['G1', 'G2'],
};

/** Lettera senza accento e segno da aggiungere sopra (`dots` = dieresi, `acute` = accento acuto, `grave`, `circ`, `tilde`). */
function split(ch: string): [string, 'dots' | 'acute' | 'grave' | 'circ' | 'tilde' | null] {
  const base = ch.normalize('NFD');
  const letter = base.charAt(0).toUpperCase();
  const mark = base.slice(1);
  if (mark.includes('̈')) return [letter, 'dots'];
  if (mark.includes('́')) return [letter, 'acute'];
  if (mark.includes('̀')) return [letter, 'grave'];
  if (mark.includes('̂')) return [letter, 'circ'];
  if (mark.includes('̃')) return [letter, 'tilde'];
  return [letter, null];
}

export interface SegTextStyle {
  /** Colore dei segmenti accesi. */
  on: string;
  /** Colore dei segmenti spenti (appena visibili). */
  off: string;
  /** Colore del bagliore. */
  glow: string;
}

const TOP = 4.5; // spazio per gli accenti sopra la cella
const CELL_W = 10;
const GAP = 4.2; // tra un carattere e l'altro
const SPACE = 6; // larghezza di uno spazio

/** L'SVG della scritta `text` a 14 segmenti. */
export function segTextSvg(text: string, style: SegTextStyle): string {
  const chars = [...text.toUpperCase()];
  let x = 2;
  const lit: string[] = [];
  const dim: string[] = [];
  const marks: string[] = [];
  for (const ch of chars) {
    if (ch === ' ') {
      x += SPACE;
      continue;
    }
    const [letter, mark] = split(ch);
    const on = GLYPHS[letter];
    if (!on) {
      x += SPACE;
      continue;
    }
    for (const s of ALL) {
      const [x1, y1, x2, y2] = SEGMENTS[s];
      const line = `<line x1="${(x + x1).toFixed(2)}" y1="${(TOP + y1).toFixed(2)}" x2="${(x + x2).toFixed(2)}" y2="${(TOP + y2).toFixed(2)}"/>`;
      (on.includes(s) ? lit : dim).push(line);
    }
    const cx = x + CELL_W / 2;
    if (mark === 'dots') marks.push(`<circle cx="${cx - 2.2}" cy="1.6" r="0.95"/><circle cx="${cx + 2.2}" cy="1.6" r="0.95"/>`);
    else if (mark === 'acute') marks.push(`<line x1="${cx - 1}" y1="3" x2="${cx + 1.6}" y2="0.4" stroke-width="1.1" stroke-linecap="round"/>`);
    else if (mark === 'grave') marks.push(`<line x1="${cx + 1}" y1="3" x2="${cx - 1.6}" y2="0.4" stroke-width="1.1" stroke-linecap="round"/>`);
    else if (mark === 'circ') marks.push(`<polyline points="${cx - 2},2.8 ${cx},0.6 ${cx + 2},2.8" fill="none" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round"/>`);
    else if (mark === 'tilde') marks.push(`<path d="M${cx - 2.4} 2.4 q1.2 -2 2.4 0 t2.4 0" fill="none" stroke-width="1.1" stroke-linecap="round"/>`);
    x += CELL_W + GAP;
  }
  const width = Math.max(4, x - GAP + 2);
  const height = TOP + 16 + 2;
  // leggera inclinazione come le cifre dei display delle radio
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="-2 -1 ${(width + 4).toFixed(2)} ${height + 2}">` +
    `<defs><filter id="g" x="-20%" y="-40%" width="140%" height="180%"><feGaussianBlur stdDeviation="0.7" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter></defs>` +
    `<g transform="skewX(-7) translate(${(height * 0.12).toFixed(2)} 0)">` +
    `<g stroke="${style.off}" stroke-width="1.35" stroke-linecap="round">${dim.join('')}</g>` +
    `<g stroke="${style.on}" stroke-width="1.35" stroke-linecap="round" filter="url(#g)" style="color:${style.glow}">${lit.join('')}</g>` +
    `<g stroke="${style.on}" fill="${style.on}" filter="url(#g)">${marks.join('')}</g>` +
    `</g></svg>`
  );
}

/** La scritta come `url(...)` per i CSS (immagine di sfondo). */
export function segTextCssUrl(text: string, style: SegTextStyle): string {
  return `url("data:image/svg+xml,${encodeURIComponent(segTextSvg(text, style))}")`;
}
