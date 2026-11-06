import { create } from 'zustand';
import { COLS, ROWS, arrange, type Cell } from '../components/GridBoard';
import { browserStorage } from './theme';

/**
 * La disposizione della pagina principale a blocchi (griglia di 12 colonne per 24 righe che riempie la finestra): ogni finestra ha la sua
 * posizione e la sua misura in celle. Si cambia trascinando (barra del titolo) e ridimensionando (angolo in basso a destra) finche'
 * `EDIT_LAYOUT` (lib/bottomLayout.ts) e' acceso; poi le misure salvate restano fisse. Chiave `aurora-board`, sincronizzata come le altre.
 */
export const BOARD_IDS = ['all', 'log', 'cluster', 'station', 'cq', 'waterfall', 'tx', 'qso'] as const;
export type BoardId = (typeof BOARD_IDS)[number];

const KEY = 'aurora-board';

/** Come stavano prima: attivita' di banda, log, cluster e comandi in alto; cascata, trasmissione, QSO e CQ in basso. */
export function defaultBoard(): Record<BoardId, Cell> {
  return {
    all: { x: 0, y: 0, w: 3, h: 13 },
    log: { x: 3, y: 0, w: 3, h: 13 },
    cluster: { x: 6, y: 0, w: 4, h: 13 },
    station: { x: 10, y: 0, w: 2, h: 13 },
    waterfall: { x: 0, y: 13, w: 5, h: 11 },
    tx: { x: 5, y: 13, w: 3, h: 11 },
    qso: { x: 8, y: 13, w: 2, h: 11 },
    cq: { x: 10, y: 13, w: 2, h: 11 },
  };
}

function valid(c: unknown): c is Cell {
  if (typeof c !== 'object' || c === null) return false;
  const { x, y, w, h } = c as Record<string, unknown>;
  return [x, y, w, h].every((n) => typeof n === 'number' && Number.isFinite(n)) && (w as number) >= 1 && (h as number) >= 1 && (x as number) >= 0 && (y as number) >= 0 && (x as number) + (w as number) <= COLS && (y as number) + (h as number) <= ROWS * 4;
}

export function readBoard(storage: Pick<Storage, 'getItem'> | null): Record<BoardId, Cell> {
  const out = defaultBoard();
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return out;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    for (const id of BOARD_IDS) if (valid(parsed[id])) out[id] = parsed[id] as Cell;
  } catch {
    return defaultBoard();
  }
  return out;
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
    // senza gravita': ogni finestra resta dove l'hai lasciata (solo le sovrapposizioni si risolvono spingendo giu')
    const next = { ...defaultBoard(), ...arrange(cells, undefined, false) } as Record<BoardId, Cell>;
    save(next);
    set({ cells: next });
  },
  reset() {
    const next = defaultBoard();
    save(next);
    set({ cells: next });
  },
}));
