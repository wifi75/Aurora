import type { CSSProperties } from 'react';
import { create } from 'zustand';
import { LABEL_IDS, type LabelId } from './dxccCard';
import { browserStorage } from './theme';

/** Le colonne della scheda DXCC, nell'ordine della scheda (le stesse del core). */
export const DXCC_BANDS = ['160m', '80m', '60m', '40m', '30m', '20m', '17m', '15m', '12m', '10m', '6m', '4m', 'U'] as const;

/** Le righe (modi) della scheda. */
export const DXCC_ROWS = ['FT8', 'FT4', 'DIG'] as const;
export type DxccRow = (typeof DXCC_ROWS)[number];

/** I tre aspetti di una casella: lavorato/confermato, lavorato non confermato, mai lavorato. */
export const CELL_KINDS = ['worked', 'unconfirmed', 'none'] as const;
export type CellKindKey = (typeof CELL_KINDS)[number];

/** I quattro colori di un'etichetta: sfondo e testo da accesa, sfondo e testo da spenta. */
export const LABEL_FIELDS = ['onBg', 'onInk', 'offBg', 'offInk'] as const;
export type LabelField = (typeof LABEL_FIELDS)[number];
export type LabelColors = Partial<Record<LabelField, string>>;

/** Le scelte della scheda DXCC; un colore assente vuol dire «di fabbrica» (il token del tema, chiaro o scuro). */
export interface DxccPrefs {
  /** Le bande mostrate (di base tutte). */
  bands: string[];
  /** Le righe mostrate: FT8 e FT4 si, DIG no. */
  rows: Record<DxccRow, boolean>;
  /** Colori delle caselle, per modo. */
  cells: Record<DxccRow, Partial<Record<CellKindKey, string>>>;
  /** Colori delle sei etichette. */
  labels: Record<LabelId, LabelColors>;
  /** Le didascalie sotto le etichette. */
  captions: boolean;
  /** Il totale dei paesi lavorati (angolo in alto a sinistra e riga in fondo). */
  total: boolean;
  /** Il campo «Prova un nominativo». */
  tryField: boolean;
  /** Dimensione compatta invece che normale. */
  compact: boolean;
}

const KEY = 'aurora-dxcc-card';
const HEX = /^#[0-9a-fA-F]{6}$/;

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

/** Nessun colore personalizzato: tutti i valori di fabbrica. */
export function factoryColors(): Pick<DxccPrefs, 'cells' | 'labels'> {
  const labels = {} as Record<LabelId, LabelColors>;
  for (const id of LABEL_IDS) labels[id] = {};
  return { cells: { FT8: {}, FT4: {}, DIG: {} }, labels };
}

export function defaultDxccPrefs(): DxccPrefs {
  return {
    bands: [...DXCC_BANDS],
    rows: { FT8: true, FT4: true, DIG: false },
    ...factoryColors(),
    captions: true,
    total: true,
    tryField: true,
    compact: false,
  };
}

function hex(value: unknown): string | undefined {
  return typeof value === 'string' && HEX.test(value) ? value.toLowerCase() : undefined;
}

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

/** Legge le scelte salvate; quello che manca o non e' valido torna al valore di fabbrica. */
export function readDxccPrefs(storage: ReadableStorage): DxccPrefs {
  const prefs = defaultDxccPrefs();
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return prefs;
    const p = JSON.parse(raw) as Record<string, unknown>;
    if (Array.isArray(p.bands)) {
      const chosen = DXCC_BANDS.filter((b) => (p.bands as unknown[]).includes(b));
      if (chosen.length > 0) prefs.bands = chosen;
    }
    if (isRecord(p.rows)) for (const r of DXCC_ROWS) if (typeof p.rows[r] === 'boolean') prefs.rows[r] = p.rows[r];
    if (!DXCC_ROWS.some((r) => prefs.rows[r])) prefs.rows = defaultDxccPrefs().rows;
    if (isRecord(p.cells)) {
      for (const r of DXCC_ROWS) {
        const row = p.cells[r];
        if (!isRecord(row)) continue;
        for (const k of CELL_KINDS) {
          const c = hex(row[k]);
          if (c) prefs.cells[r][k] = c;
        }
      }
    }
    if (isRecord(p.labels)) {
      for (const id of LABEL_IDS) {
        const l = p.labels[id];
        if (!isRecord(l)) continue;
        for (const f of LABEL_FIELDS) {
          const c = hex(l[f]);
          if (c) prefs.labels[id][f] = c;
        }
      }
    }
    for (const key of ['captions', 'total', 'tryField', 'compact'] as const) if (typeof p[key] === 'boolean') prefs[key] = p[key];
  } catch {
    return defaultDxccPrefs();
  }
  return prefs;
}

export function saveDxccPrefs(storage: WritableStorage, prefs: DxccPrefs): void {
  try {
    storage?.setItem(KEY, JSON.stringify(prefs));
  } catch {
    // senza archivio le scelte non vengono ricordate, ma la scheda funziona lo stesso
  }
}

/** Testo nero o bianco, a seconda di quale si legge meglio sullo sfondo scelto. */
export function inkFor(color: string): string {
  const m = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(color);
  if (!m) return '#000000';
  const lin = (h: string) => {
    const c = parseInt(h, 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  const lum = 0.2126 * lin(m[1]!) + 0.7152 * lin(m[2]!) + 0.0722 * lin(m[3]!);
  return lum > 0.35 ? '#000000' : '#ffffff';
}

/** Lo stile di una casella per il modo e l'aspetto dati; senza colore scelto resta quello del tema. */
export function cellStyle(prefs: DxccPrefs, row: string, kind: CellKindKey): CSSProperties | undefined {
  const bg = prefs.cells[row as DxccRow]?.[kind];
  return bg ? { background: bg, color: inkFor(bg) } : undefined;
}

/** Lo stile di un'etichetta, accesa o spenta. */
export function labelStyle(prefs: DxccPrefs, id: LabelId, on: boolean): CSSProperties | undefined {
  const c = prefs.labels[id];
  const bg = on ? c.onBg : c.offBg;
  const ink = on ? c.onInk : c.offInk;
  if (!bg && !ink) return undefined;
  const style: CSSProperties = {};
  if (bg) {
    style.background = bg;
    style.borderColor = bg;
    if (!ink) style.color = inkFor(bg);
  }
  if (ink) style.color = ink;
  return style;
}

/** Ci sono colori personalizzati? */
export function hasCustomColors(prefs: DxccPrefs): boolean {
  return DXCC_ROWS.some((r) => Object.keys(prefs.cells[r]).length > 0) || LABEL_IDS.some((id) => Object.keys(prefs.labels[id]).length > 0);
}

interface DxccPrefsState {
  prefs: DxccPrefs;
  set(change: Partial<DxccPrefs>): void;
  /** Ripristina i colori di fabbrica (bande, righe e aspetto restano). */
  resetColors(): void;
  reset(): void;
}

export const useDxccPrefs = create<DxccPrefsState>((set, get) => {
  const commit = (next: DxccPrefs) => {
    saveDxccPrefs(browserStorage(), next);
    set({ prefs: next });
  };
  return {
    prefs: readDxccPrefs(browserStorage()),
    set: (change) => commit({ ...get().prefs, ...change }),
    resetColors: () => commit({ ...get().prefs, ...factoryColors() }),
    reset: () => commit(defaultDxccPrefs()),
  };
});
