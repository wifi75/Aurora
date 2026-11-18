import { create } from 'zustand';
import { browserStorage } from './theme';

/**
 * La riga in basso (cascata, Trasmissione, QSO): altezza in pixel e larghezza di ognuna come quota del totale. Si regolano trascinando
 * il bordo alto e i divisori fra le finestre; la scelta resta nel browser e sul server (chiave `aurora-bottom`, come le altre dell'interfaccia).
 */
export type BottomKey = 'waterfall' | 'tx' | 'qso';

/**
 * Disposizione modificabile: con la scelta accesa (Impostazioni › Aspetto › «Finestre della pagina FT8») le finestre della pagina si spostano
 * (barra del titolo), si ridimensionano (angolo) e la riga in basso si allarga (divisori) e si alza (bordo alto). Spenta di fabbrica: restano le
 * misure salvate come fisse e i comandi di regolazione spariscono. La scelta resta su questo dispositivo.
 */
const EDIT_KEY = 'aurora-edit-layout';

function readEdit(): boolean {
  try {
    return browserStorage()?.getItem(EDIT_KEY) === '1';
  } catch {
    return false;
  }
}

interface EditState {
  on: boolean;
  set(on: boolean): void;
}

export const useEditLayout = create<EditState>((set) => ({
  on: readEdit(),
  set(on) {
    try {
      browserStorage()?.setItem(EDIT_KEY, on ? '1' : '0');
    } catch {
      // senza archivio la scelta vale finche' resta aperta la finestra
    }
    set({ on });
  },
}));

export interface BottomLayout {
  heightPx: number;
  /** Da sinistra a destra: quale finestra sta in ogni posto. */
  order: [BottomKey, BottomKey, BottomKey];
  /** Quote di larghezza per posto, da sinistra (la somma fa 1). */
  widths: [number, number, number];
}

const KEY = 'aurora-bottom';
export const BOTTOM_MIN_HEIGHT = 160;
export const BOTTOM_MAX_HEIGHT = 640;
const MIN_SHARE = 0.12;

export function defaultBottom(): BottomLayout {
  return { heightPx: 288, order: ['waterfall', 'tx', 'qso'], widths: [0.42, 0.29, 0.29] };
}

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** Porta le tre quote a somma 1, ognuna almeno `MIN_SHARE`. */
export function normalizeWidths(w: readonly number[]): [number, number, number] {
  const raw = [0, 1, 2].map((i) => (Number.isFinite(w[i]) ? Math.max(MIN_SHARE, w[i]!) : defaultBottom().widths[i]!));
  const sum = raw[0]! + raw[1]! + raw[2]!;
  return [raw[0]! / sum, raw[1]! / sum, raw[2]! / sum];
}

export function readBottom(storage: Pick<Storage, 'getItem'> | null): BottomLayout {
  const out = defaultBottom();
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return out;
    const parsed = JSON.parse(raw) as { heightPx?: unknown; widths?: unknown; order?: unknown };
    if (typeof parsed.heightPx === 'number' && Number.isFinite(parsed.heightPx)) out.heightPx = clamp(Math.round(parsed.heightPx), BOTTOM_MIN_HEIGHT, BOTTOM_MAX_HEIGHT);
    if (Array.isArray(parsed.widths) && parsed.widths.length === 3) out.widths = normalizeWidths(parsed.widths as number[]);
    const order = parsed.order;
    if (Array.isArray(order) && order.length === 3 && (['waterfall', 'tx', 'qso'] as const).every((k) => order.includes(k))) out.order = order as BottomLayout['order'];
  } catch {
    return defaultBottom();
  }
  return out;
}

function save(layout: BottomLayout): void {
  try {
    browserStorage()?.setItem(KEY, JSON.stringify(layout));
  } catch {
    // senza archivio la scelta vale fino alla chiusura della pagina
  }
}

interface BottomState {
  layout: BottomLayout;
  set(change: Partial<BottomLayout>): void;
  reset(): void;
}

export const useBottom = create<BottomState>((set, get) => ({
  layout: readBottom(browserStorage()),
  set(change) {
    const current = get().layout;
    const next: BottomLayout = {
      heightPx: change.heightPx === undefined ? current.heightPx : clamp(Math.round(change.heightPx), BOTTOM_MIN_HEIGHT, BOTTOM_MAX_HEIGHT),
      order: change.order ?? current.order,
      widths: change.widths ? normalizeWidths(change.widths) : current.widths,
    };
    save(next);
    set({ layout: next });
  },
  reset() {
    const next = defaultBottom();
    save(next);
    set({ layout: next });
  },
}));
