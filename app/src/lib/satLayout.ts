import type { DashItem, DashSpec } from './dashGrid';
import { loadDashLayout, parseDashLayout, saveDashLayout, serializeDashLayout, type DashLayoutRules, type StoredLayout } from './dashLayout';

export type { StoredLayout } from './dashLayout';

/** Memoria della disposizione delle schede della pagina Satelliti. */
export const SAT_LAYOUT_KEY = 'aurora.sat.layout';
/** Versione dello schema salvato: se cambia, la disposizione salvata si scarta. */
export const SAT_LAYOUT_VERSION = 5;

export const SAT_WIDGET_IDS = ['map', 'list', 'data', 'pass', 'ctl', 'all', 'info'] as const;
export type SatWidgetId = (typeof SAT_WIDGET_IDS)[number];

/** Dimensioni minime (colonne x righe) di ogni scheda, le piu' piccole che il contenuto sopporta (oltre scorre dentro la scheda); la cartina ha le proporzioni fisse 2/1. L'altezza vera segue il contenuto. */
export const SAT_MINS: Record<SatWidgetId, Pick<DashSpec, 'minW' | 'minH'>> = {
  map: { minW: 3, minH: 3 },
  list: { minW: 2, minH: 3 },
  data: { minW: 2, minH: 3 },
  pass: { minW: 3, minH: 3 },
  ctl: { minW: 2, minH: 3 },
  all: { minW: 3, minH: 3 },
  info: { minW: 2, minH: 2 },
};

/** Larghezza massima (colonne) delle schede con poco contenuto: non si allargano oltre, nemmeno col riempimento a destra. */
export const SAT_MAX_COLS: Partial<Record<SatWidgetId, number>> = { data: 5, list: 5, ctl: 5, info: 5, pass: 8, all: 8 };

/** Altezza massima (righe) che una scheda prende dal contenuto: oltre, scorre dentro la scheda. */
export const SAT_MAX_ROWS: Record<SatWidgetId, number> = { map: 40, list: 10, data: 12, pass: 9, ctl: 22, all: 9, info: 10 };

/** Disposizione di partenza: a sinistra la cartina (8 colonne) e sotto i due passaggi affiancati (4 + 4); a destra, in 4 colonne, dati, elenco, Doppler e rotore, informazioni. Le altezze vere seguono il contenuto. */
export function defaultLayout(): DashItem[] {
  return [
    { id: 'map', x: 0, y: 0, w: 8, h: 3 },
    { id: 'data', x: 8, y: 0, w: 4, h: 3 },
    { id: 'list', x: 8, y: 5, w: 4, h: 3 },
    { id: 'ctl', x: 8, y: 11, w: 4, h: 3 },
    { id: 'info', x: 8, y: 20, w: 4, h: 2 },
    { id: 'pass', x: 0, y: 14, w: 4, h: 3 },
    { id: 'all', x: 4, y: 14, w: 4, h: 3 },
  ];
}

const RULES: DashLayoutRules = {
  version: SAT_LAYOUT_VERSION,
  accepts: (id) => (SAT_WIDGET_IDS as readonly string[]).includes(id),
  mins: (id) => SAT_MINS[id as SatWidgetId],
  maxCols: (id) => SAT_MAX_COLS[id as SatWidgetId] ?? 12,
  exact: SAT_WIDGET_IDS,
};

/** Legge e controlla la disposizione salvata; null se manca, e' corrotta, di un'altra versione o non ha tutte e sole le schede note. */
export const parseLayout = (raw: string | null): StoredLayout | null => parseDashLayout(raw, RULES);

export const serializeLayout = (l: StoredLayout): string => serializeDashLayout(l, RULES);

/** La disposizione salvata nel browser, o quella di partenza. */
export function loadLayout(): StoredLayout {
  return loadDashLayout(SAT_LAYOUT_KEY, RULES) ?? { items: defaultLayout(), locked: false };
}

export function saveLayout(l: StoredLayout): void {
  saveDashLayout(SAT_LAYOUT_KEY, l, RULES);
}
