/**
 * Griglia a colonne per le schede spostabili e ridimensionabili: funzioni pure, nessun accesso al DOM.
 * Ogni scheda e' un rettangolo {id, x, y, w, h} in celle (colonne e righe). Dopo ogni modifica la disposizione
 * si risistema: le schede salgono fin dove non urtano (niente vuoti sopra), si allargano verso destra (e verso
 * sinistra) nello spazio libero e quelle affiancate nella stessa banda prendono la stessa altezza.
 */

export const DASH_COLS = 12;

export interface DashItem {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
  /** Altezza scelta dall'utente (puo' essere piu' bassa del contenuto, che allora scorre); senza, l'altezza segue il contenuto. */
  userH?: boolean;
}

export interface DashSpec {
  /** Colonne minime. */
  minW: number;
  /** Righe minime. */
  minH: number;
  /** Colonne massime: ne' il riempimento a destra ne' il ridimensionamento allargano la scheda oltre. */
  maxW?: number;
  /** Altezza (in righe) del contenuto: la scheda non scende mai sotto questa, cosi' niente testo tagliato; senza di essa vale `minH`. */
  rows?: number;
  /** Se c'e', l'altezza (in righe) e' funzione della larghezza (in colonne): la scheda ha proporzioni fisse, non si allarga da sola e non si pareggia. */
  hOf?: (w: number) => number;
}

export type DashSpecs = Record<string, DashSpec>;

const DEFAULT_SPEC: DashSpec = { minW: 1, minH: 1 };
const specOf = (specs: DashSpecs, id: string): DashSpec => specs[id] ?? DEFAULT_SPEC;

const toInt = (n: number, fallback: number): number => (Number.isFinite(n) ? Math.round(n) : fallback);
const clamp = (n: number, lo: number, hi: number): number => Math.max(lo, Math.min(hi, n));

/** Le due schede si sovrappongono? (i bordi che si toccano non contano) */
export function overlaps(a: DashItem, b: DashItem): boolean {
  return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
}

/** Porta una scheda dentro la griglia: interi, dimensioni minime e massime rispettate, altezza dalla larghezza se ha proporzioni fisse. */
export function clampItem(item: DashItem, specs: DashSpecs, cols: number = DASH_COLS): DashItem {
  const sp = specOf(specs, item.id);
  const w = clamp(toInt(item.w, sp.minW), Math.min(sp.minW, cols), Math.max(Math.min(sp.minW, cols), Math.min(sp.maxW ?? cols, cols)));
  const x = clamp(toInt(item.x, 0), 0, cols - w);
  const y = Math.max(0, toInt(item.y, 0));
  const user = item.userH === true && !sp.hOf;
  const h = sp.hOf ? Math.max(1, toInt(sp.hOf(w), sp.minH)) : user ? Math.max(sp.minH, toInt(item.h, sp.minH)) : Math.max(sp.minH, sp.rows ?? toInt(item.h, sp.minH));
  return user ? { id: item.id, x, y, w, h, userH: true } : { id: item.id, x, y, w, h };
}

const byPosition = (a: DashItem, b: DashItem) => a.y - b.y || a.x - b.x || a.id.localeCompare(b.id);

/**
 * Compattazione verticale: ogni scheda, nell'ordine dall'alto a sinistra, se urta una gia' sistemata scende sotto di essa,
 * poi sale finche' non tocca un'altra scheda o il bordo alto. `firstId` (se c'e') resta dov'e' e le altre si adattano a lei.
 */
export function compact(items: readonly DashItem[], firstId?: string): DashItem[] {
  const placed: DashItem[] = [];
  const put = (it: DashItem) => {
    const at = (y: number) => placed.some((o) => overlaps({ ...it, y }, o));
    let y = it.y;
    while (at(y)) y += 1;
    while (y > 0 && !at(y - 1)) y -= 1;
    placed.push({ ...it, y });
  };
  const first = firstId === undefined ? undefined : items.find((i) => i.id === firstId);
  if (first) placed.push({ ...first });
  for (const it of [...items].sort(byPosition)) if (it !== first) put(it);
  const index = new Map(items.map((it, i) => [it.id, i]));
  return placed.sort((a, b) => (index.get(a.id) ?? 0) - (index.get(b.id) ?? 0));
}

const growable = (it: DashItem, specs: DashSpecs) => !specOf(specs, it.id).hOf;

/** Quante colonne puo' prendere ancora la scheda verso destra senza urtare nessuno. */
function roomRight(cur: readonly DashItem[], it: DashItem, cols: number): number {
  let n = 0;
  while (it.x + it.w + n < cols && !cur.some((o) => o !== it && overlaps({ ...it, w: it.w + n + 1 }, o))) n += 1;
  return n;
}

/** Quante colonne puo' prendere ancora la scheda verso sinistra senza urtare nessuno. */
function roomLeft(cur: readonly DashItem[], it: DashItem): number {
  let n = 0;
  while (it.x - n > 0 && !cur.some((o) => o !== it && overlaps({ ...it, x: it.x - n - 1, w: it.w + n + 1 }, o))) n += 1;
  return n;
}

/** La scheda piu' vicina a destra che le sta di fronte (si sovrappone in altezza). */
function rightNeighbour(cur: readonly DashItem[], it: DashItem): DashItem | undefined {
  let best: DashItem | undefined;
  for (const o of cur) {
    if (o === it || o.x < it.x + it.w || o.y >= it.y + it.h || it.y >= o.y + o.h) continue;
    if (!best || o.x < best.x) best = o;
  }
  return best;
}

/**
 * Riempimento a destra: le schede che possono allargarsi prendono le colonne libere fino al bordo destro o alla scheda vicina;
 * se le colonne libere stanno tra due schede si dividono, quella a destra si allarga verso sinistra.
 * Le schede a proporzioni fisse non cambiano; `activeId` (la scheda che l'utente sta modificando) si allarga per ultima.
 */
export function fillRight(items: readonly DashItem[], specs: DashSpecs, cols: number = DASH_COLS, activeId?: string): DashItem[] {
  const cur = items.map((i) => ({ ...i }));
  /** Colonne che la scheda puo' ancora prendere prima del suo massimo. */
  const cap = (it: DashItem) => Math.max(0, Math.min(specOf(specs, it.id).maxW ?? cols, cols) - it.w);
  const pass = (withActive: boolean) => {
    const ok = (it: DashItem) => growable(it, specs) && (withActive || it.id !== activeId);
    // colonne libere tra due schede: si dividono a meta'
    for (const it of [...cur].sort((a, b) => a.x - b.x || a.y - b.y)) {
      const other = rightNeighbour(cur, it);
      if (!other) continue;
      const gap = roomRight(cur, it, cols);
      if (gap <= 0) continue;
      const otherShare = ok(other) ? Math.min(roomLeft(cur, other), cap(other), gap - (ok(it) ? Math.floor(gap / 2) : 0)) : 0;
      const ownShare = ok(it) ? Math.min(gap - otherShare, cap(it)) : 0;
      it.w += ownShare;
      other.x -= otherShare;
      other.w += otherShare;
    }
    // il resto: verso destra, poi verso sinistra
    for (const it of cur) {
      if (!ok(it)) continue;
      it.w += Math.min(roomRight(cur, it, cols), cap(it));
      const l = Math.min(roomLeft(cur, it), cap(it));
      it.x -= l;
      it.w += l;
    }
  };
  pass(false);
  pass(true);
  return cur;
}

/** Differenza massima (in righe) fra schede affiancate perche' si pareggino: oltre, restano com'e'. */
export const EQUALIZE_MAX_DIFF = 3;

/**
 * Pareggio delle altezze: le schede con lo stesso bordo alto (la stessa banda) prendono la stessa altezza, quella della piu' alta
 * (o quella di `priorityId`, se e' nella banda), ma solo se la differenza e' piccola (al massimo 3 righe): una scheda molto piu' bassa
 * non si allunga per forza. Le schede a proporzioni fisse restano fuori e nessuna scende sotto la propria altezza minima.
 */
export function equalizeRows(items: readonly DashItem[], specs: DashSpecs, priorityId?: string): DashItem[] {
  const out = items.map((i) => ({ ...i }));
  const bands = new Map<number, DashItem[]>();
  for (const it of out) {
    if (!growable(it, specs)) continue;
    bands.set(it.y, [...(bands.get(it.y) ?? []), it]);
  }
  const floor = (it: DashItem) => Math.max(specOf(specs, it.id).minH, specOf(specs, it.id).rows ?? 0);
  for (const band of bands.values()) {
    if (band.length < 2) continue;
    const lead = band.find((i) => i.id === priorityId);
    const target = lead ? lead.h : Math.max(...band.map((i) => i.h));
    for (const it of band) if (Math.abs(target - it.h) <= EQUALIZE_MAX_DIFF) it.h = Math.max(target, floor(it));
  }
  return out;
}

export interface NormalizeOptions {
  cols?: number;
  /** La scheda appena spostata: resta dov'e' e le altre si adattano. */
  firstId?: string;
  /** La scheda che si sta ridimensionando: si allarga per ultima e decide l'altezza della banda. */
  activeId?: string;
  priorityId?: string;
  /** Riempimento automatico a destra e pareggio delle altezze: spento di default (un vuoto accanto a una scheda piccola va bene, una scheda gonfiata no). */
  fill?: boolean;
}

const keyOf = (items: readonly DashItem[]) => items.map((i) => `${i.id}:${i.x},${i.y},${i.w},${i.h}`).join('|');

/**
 * Sistema una disposizione qualsiasi: limiti e compattazione verticale. Con `fill` (spento di default) si aggiungono
 * il riempimento a destra e il pareggio delle altezze, ripetuti finche' la disposizione non cambia piu'.
 */
export function normalize(items: readonly DashItem[], specs: DashSpecs, opts: NormalizeOptions = {}): DashItem[] {
  const cols = opts.cols ?? DASH_COLS;
  // la scheda appena spostata resta dov'e', le altre si adattano; poi tutto sale verso l'alto
  let cur = compact(
    compact(
      items.map((i) => clampItem(i, specs, cols)),
      opts.firstId,
    ),
  );
  if (!opts.fill) return cur;
  for (let i = 0; i < 8; i += 1) {
    const before = keyOf(cur);
    cur = fillRight(cur, specs, cols, opts.activeId);
    cur = equalizeRows(cur, specs, opts.priorityId);
    cur = compact(cur);
    if (keyOf(cur) === before) break;
  }
  return cur;
}

/** Sposta una scheda nella cella (x, y): le altre si spostano per farle posto e tutto si risistema. Una scheda che non c'e' non cambia nulla. */
export function moveItem(items: readonly DashItem[], id: string, x: number, y: number, specs: DashSpecs, cols: number = DASH_COLS): DashItem[] {
  const from = items.find((i) => i.id === id);
  if (!from) return normalize(items, specs, { cols });
  const target = clampItem({ ...from, x, y }, specs, cols);
  // lasciata sopra una sola scheda: le due si scambiano il posto, se cosi' non urtano nessuno
  const hit = items.filter((o) => o.id !== id && overlaps(target, o));
  if (hit.length === 1 && hit[0]) {
    const other = hit[0];
    const there = clampItem({ ...other, x: from.x, y: from.y }, specs, cols);
    const rest = items.filter((o) => o.id !== id && o.id !== other.id);
    if (!overlaps(there, target) && !rest.some((o) => overlaps(there, o) || overlaps(target, o))) {
      const swapped = items.map((i) => (i.id === id ? target : i.id === other.id ? there : i));
      return normalize(swapped, specs, { cols, firstId: id });
    }
  }
  const moved = items.map((i) => (i.id === id ? { ...i, x, y } : i));
  return normalize(moved, specs, { cols, firstId: id });
}

/** Cambia la dimensione di una scheda (l'altezza scelta resta, anche sotto il contenuto; rispettando minimi, massimi, bordo destro e proporzioni fisse): le schede che urta scendono e tutto si compatta. */
export function resizeItem(items: readonly DashItem[], id: string, w: number, h: number, specs: DashSpecs, cols: number = DASH_COLS): DashItem[] {
  const from = items.find((i) => i.id === id);
  if (!from) return normalize(items, specs, { cols });
  const x = clamp(toInt(from.x, 0), 0, cols - 1);
  const grown = clampItem({ ...from, x, w: clamp(toInt(w, from.w), 1, cols - x), h, userH: true }, specs, cols);
  const resized = items.map((i) => (i.id === id ? grown : i));
  return normalize(resized, specs, { cols, firstId: id, activeId: id, priorityId: id });
}

/** Cambia solo la larghezza (a scatti di colonna, entro minimo e massimo della scheda): l'altezza segue il contenuto. */
export function resizeWidth(items: readonly DashItem[], id: string, w: number, specs: DashSpecs, cols: number = DASH_COLS): DashItem[] {
  const from = items.find((i) => i.id === id);
  return resizeItem(items, id, w, from ? from.h : 1, specs, cols);
}

/** Da una misura in pixel al numero di celle (colonne o righe) piu' vicino, con lo spazio fra le celle. */
export function snapCells(px: number, cell: number, gap: number): number {
  return Math.max(1, Math.round((px + gap) / (cell + gap)));
}
