export type PanelId = 'waterfall' | 'station' | 'tx' | 'cq' | 'all';
export type Zone = 'left' | 'right';

export interface Layout {
  left: PanelId[];
  right: PanelId[];
  /** Altezza scelta dall'utente per pannello, in pixel; assente = automatica. */
  heights: Partial<Record<PanelId, number>>;
  /** Quota di larghezza della colonna sinistra. */
  split: number;
}

export const ALL_PANELS: readonly PanelId[] = ['waterfall', 'station', 'tx', 'cq', 'all'];
const KEY = 'aurora-layout';
const MIN_HEIGHT = 80;
const MAX_HEIGHT = 2000;
const MIN_SPLIT = 0.25;
const MAX_SPLIT = 0.85;

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

export function defaultLayout(): Layout {
  return { left: ['all', 'waterfall'], right: ['station', 'tx', 'cq'], heights: {}, split: 0.6 };
}

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** Sposta un pannello in una colonna a una posizione; ogni pannello resta presente una volta sola. */
export function movePanel(layout: Layout, id: PanelId, zone: Zone, index: number): Layout {
  const left = layout.left.filter((p) => p !== id);
  const right = layout.right.filter((p) => p !== id);
  const target = zone === 'left' ? left : right;
  target.splice(clamp(Math.trunc(index), 0, target.length), 0, id);
  return { ...layout, left, right };
}

export function resizePanel(layout: Layout, id: PanelId, height: number): Layout {
  return { ...layout, heights: { ...layout.heights, [id]: Math.round(clamp(height, MIN_HEIGHT, MAX_HEIGHT)) } };
}

export function clearHeight(layout: Layout, id: PanelId): Layout {
  const { [id]: _removed, ...heights } = layout.heights;
  return { ...layout, heights };
}

export function setSplit(layout: Layout, split: number): Layout {
  return { ...layout, split: clamp(split, MIN_SPLIT, MAX_SPLIT) };
}

function validPanels(left: unknown, right: unknown): left is PanelId[] {
  if (!Array.isArray(left) || !Array.isArray(right)) return false;
  const all = [...left, ...right];
  return all.length === ALL_PANELS.length && ALL_PANELS.every((p) => all.filter((x) => x === p).length === 1);
}

export function readLayout(storage: ReadableStorage): Layout {
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return defaultLayout();
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    if (!validPanels(parsed.left, parsed.right)) return defaultLayout();
    const heights: Layout['heights'] = {};
    if (typeof parsed.heights === 'object' && parsed.heights !== null) {
      for (const id of ALL_PANELS) {
        const h = (parsed.heights as Record<string, unknown>)[id];
        if (typeof h === 'number' && Number.isFinite(h)) heights[id] = Math.round(clamp(h, MIN_HEIGHT, MAX_HEIGHT));
      }
    }
    const split = typeof parsed.split === 'number' && Number.isFinite(parsed.split) ? clamp(parsed.split, MIN_SPLIT, MAX_SPLIT) : defaultLayout().split;
    return { left: parsed.left as PanelId[], right: parsed.right as PanelId[], heights, split };
  } catch {
    return defaultLayout();
  }
}

export function saveLayout(storage: WritableStorage, layout: Layout): void {
  try {
    storage?.setItem(KEY, JSON.stringify(layout));
  } catch {
    // senza archivio la disposizione non viene ricordata, ma l'interfaccia funziona lo stesso
  }
}
