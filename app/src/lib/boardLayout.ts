import { create } from 'zustand';
import { COLS, MAX_ROWS, type Cell } from '../components/GridBoard';
import { browserStorage } from './theme';

/**
 * La disposizione della pagina principale a blocchi (griglia di 12 colonne per 24 righe che riempie la finestra): ogni finestra ha la sua
 * posizione e la sua misura in celle. Si cambia trascinando (barra del titolo) e ridimensionando (angolo in basso a destra) finche'
 * `EDIT_LAYOUT` (lib/bottomLayout.ts) e' acceso; poi le misure salvate restano fisse. Chiave `aurora-board`, sincronizzata come le altre.
 */
export const BOARD_IDS = ['all', 'log', 'cluster', 'cq', 'waterfall', 'tx'] as const;
export type BoardId = (typeof BOARD_IDS)[number];

const KEY = 'aurora-board';

/** Come stavano prima: attivita' di banda, log, cluster e comandi in alto; cascata, trasmissione, QSO e CQ in basso. */
export function defaultBoard(): Record<BoardId, Cell> {
  return {
    all: { x: 0, y: 0, w: 3, h: 13 },
    log: { x: 3, y: 0, w: 3, h: 13 },
    cluster: { x: 6, y: 0, w: 6, h: 13 },
    waterfall: { x: 0, y: 13, w: 5, h: 11 },
    tx: { x: 5, y: 13, w: 4, h: 11 },
    cq: { x: 9, y: 13, w: 3, h: 11 },
  };
}

function valid(c: unknown): c is Cell {
  if (typeof c !== 'object' || c === null) return false;
  const { x, y, w, h } = c as Record<string, unknown>;
  return [x, y, w, h].every((n) => typeof n === 'number' && Number.isFinite(n)) && (w as number) >= 1 && (h as number) >= 1 && (x as number) >= 0 && (y as number) >= 0 && (x as number) + (w as number) <= COLS && (y as number) + (h as number) <= MAX_ROWS;
}

const overlaps = (a: Cell, b: Cell) => a.x < b.x + b.w - 1e-6 && b.x < a.x + a.w - 1e-6 && a.y < b.y + b.h - 1e-6 && b.y < a.y + a.h - 1e-6;

/**
 * Nessuna finestra deve restare coperta da un'altra o fuori dallo schermo: le finestre si tengono dentro le 24 righe e, se due si
 * sovrappongono, quella che viene dopo (cascata, Trasmissione e QSO hanno la precedenza) va nel primo spazio libero della stessa misura;
 * se non c'e' posto, prova la misura di partenza e, in ultimo, torna tutta la disposizione di partenza.
 */
export function settleBoard(cells: Record<BoardId, Cell>): Record<BoardId, Cell> {
  const order: BoardId[] = ['waterfall', 'tx', 'cq', 'all', 'log', 'cluster'];
  const placed: Partial<Record<BoardId, Cell>> = {};
  const fits = (c: Cell) => c.x >= 0 && c.y >= 0 && c.x + c.w <= COLS + 1e-6 && c.y + c.h <= MAX_ROWS + 1e-6 && Object.values(placed).every((p) => !overlaps(c, p));
  const spot = (w: number, h: number): Cell | null => {
    for (let y = 0; y + h <= MAX_ROWS + 1e-6; y += 0.5) for (let x = 0; x + w <= COLS + 1e-6; x += 0.5) if (fits({ x, y, w, h })) return { x, y, w, h };
    return null;
  };
  const base = defaultBoard();
  for (const id of order) {
    const c0 = cells[id];
    const c: Cell = { ...c0, y: Math.max(0, Math.min(c0.y, MAX_ROWS - c0.h)), x: Math.max(0, Math.min(c0.x, COLS - c0.w)) };
    const found = fits(c) ? c : (spot(c0.w, c0.h) ?? spot(base[id].w, base[id].h));
    if (!found) return base;
    placed[id] = found;
  }
  return placed as Record<BoardId, Cell>;
}

export function readBoard(storage: Pick<Storage, 'getItem'> | null): Record<BoardId, Cell> {
  const out = defaultBoard();
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return out;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    for (const id of BOARD_IDS) if (valid(parsed[id])) out[id] = parsed[id] as Cell;
    // una finestra rimasta sotto la parte visibile o coperta da un'altra viene rimessa a posto
    return settleBoard(out);
  } catch {
    return defaultBoard();
  }
}

interface BoardState {
  cells: Record<BoardId, Cell>;
  set(cells: Record<string, Cell>): void;
  reset(): void;
}

function save(cells: Record<BoardId, Cell>): void {
  try {
    browserStorage()?.setItem(KEY, JSON.stringify(cells));
  } catch {
    // senza archivio la disposizione vale fino alla chiusura della pagina
  }
}

export const useBoard = create<BoardState>((set) => ({
  cells: readBoard(browserStorage()),
  set(cells) {
    // gia' sistemati da GridBoard (la finestra mossa resta dove l'hai lasciata, le altre si spostano): qui solo si tengono dentro la parte visibile
    const merged = { ...defaultBoard(), ...cells } as Record<BoardId, Cell>;
    const next = merged;
    save(next);
    set({ cells: next });
  },
  reset() {
    const next = defaultBoard();
    save(next);
    set({ cells: next });
  },
}));
