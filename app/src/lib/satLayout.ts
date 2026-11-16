import { DASH_COLS, type DashItem, type DashSpec } from './dashGrid';

/** Memoria della disposizione delle schede della pagina Satelliti. */
export const SAT_LAYOUT_KEY = 'aurora.sat.layout';
/** Versione dello schema salvato: se cambia, la disposizione salvata si scarta. */
export const SAT_LAYOUT_VERSION = 5;

export const SAT_WIDGET_IDS = ['map', 'list', 'data', 'pass', 'ctl', 'all', 'info'] as const;
export type SatWidgetId = (typeof SAT_WIDGET_IDS)[number];

/** Dimensioni minime (colonne x righe) di ogni scheda; la cartina ha le proporzioni fisse 2/1. L'altezza vera segue il contenuto. */
export const SAT_MINS: Record<SatWidgetId, Pick<DashSpec, 'minW' | 'minH'>> = {
  map: { minW: 4, minH: 4 },
  list: { minW: 3, minH: 5 },
  data: { minW: 3, minH: 4 },
  pass: { minW: 4, minH: 4 },
  ctl: { minW: 3, minH: 6 },
  all: { minW: 4, minH: 4 },
  info: { minW: 3, minH: 3 },
};

/** Larghezza massima (colonne) delle schede con poco contenuto: non si allargano oltre, nemmeno col riempimento a destra. */
export const SAT_MAX_COLS: Partial<Record<SatWidgetId, number>> = { data: 5, list: 5, ctl: 5, info: 5, pass: 8, all: 8 };

/** Altezza massima (righe) che una scheda prende dal contenuto: oltre, scorre dentro la scheda. */
export const SAT_MAX_ROWS: Record<SatWidgetId, number> = { map: 40, list: 10, data: 12, pass: 9, ctl: 22, all: 9, info: 10 };

/** Disposizione di partenza: a sinistra la cartina (8 colonne) e sotto i due passaggi affiancati (4 + 4); a destra, in 4 colonne, dati, elenco, Doppler e rotore, informazioni. Le altezze vere seguono il contenuto. */
export function defaultLayout(): DashItem[] {
  return [
    { id: 'map', x: 0, y: 0, w: 8, h: 4 },
    { id: 'data', x: 8, y: 0, w: 4, h: 4 },
    { id: 'list', x: 8, y: 5, w: 4, h: 5 },
    { id: 'ctl', x: 8, y: 11, w: 4, h: 6 },
    { id: 'info', x: 8, y: 20, w: 4, h: 3 },
    { id: 'pass', x: 0, y: 14, w: 4, h: 4 },
    { id: 'all', x: 4, y: 14, w: 4, h: 4 },
  ];
}

export interface StoredLayout {
  items: DashItem[];
  locked: boolean;
}

const isInt = (n: unknown): n is number => typeof n === 'number' && Number.isInteger(n);

/** Legge e controlla la disposizione salvata; null se manca, e' corrotta, di un'altra versione o non ha tutte e sole le schede note. */
export function parseLayout(raw: string | null): StoredLayout | null {
  if (!raw) return null;
  try {
    const data: unknown = JSON.parse(raw);
    if (typeof data !== 'object' || data === null) return null;
    const rec = data as Record<string, unknown>;
    if (rec.v !== SAT_LAYOUT_VERSION || !Array.isArray(rec.items) || rec.items.length !== SAT_WIDGET_IDS.length) return null;
    const seen = new Set<string>();
    const items: DashItem[] = [];
    for (const row of rec.items as unknown[]) {
      if (typeof row !== 'object' || row === null) return null;
      const r = row as Record<string, unknown>;
      const id = r.id;
      if (typeof id !== 'string' || !(SAT_WIDGET_IDS as readonly string[]).includes(id) || seen.has(id)) return null;
      const { x, y, w, h } = r;
      // `h` e' facoltativa: senza, l'altezza segue il contenuto
      if (!isInt(x) || !isInt(y) || !isInt(w) || (h !== undefined && !isInt(h))) return null;
      const min = SAT_MINS[id as SatWidgetId];
      if (x < 0 || y < 0 || w < min.minW || w > (SAT_MAX_COLS[id as SatWidgetId] ?? DASH_COLS) || (h !== undefined && h < min.minH) || x + w > DASH_COLS || y > 10_000 || (h !== undefined && h > 10_000)) return null;
      seen.add(id);
      items.push(h === undefined ? { id, x, y, w, h: min.minH } : { id, x, y, w, h, userH: true });
    }
    return { items, locked: rec.locked === true };
  } catch {
    return null;
  }
}

export function serializeLayout(l: StoredLayout): string {
  return JSON.stringify({ v: SAT_LAYOUT_VERSION, locked: l.locked, items: l.items.map((i) => (i.userH ? { id: i.id, x: i.x, y: i.y, w: i.w, h: i.h } : { id: i.id, x: i.x, y: i.y, w: i.w })) });
}

/** La disposizione salvata nel browser, o quella di partenza. */
export function loadLayout(): StoredLayout {
  try {
    const found = parseLayout(window.localStorage.getItem(SAT_LAYOUT_KEY));
    if (found) return found;
  } catch {
    /* memoria del browser non disponibile */
  }
  return { items: defaultLayout(), locked: false };
}

export function saveLayout(l: StoredLayout): void {
  try {
    window.localStorage.setItem(SAT_LAYOUT_KEY, serializeLayout(l));
  } catch {
    /* memoria del browser non disponibile */
  }
}
