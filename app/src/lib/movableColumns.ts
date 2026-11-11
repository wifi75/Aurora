import { browserStorage } from './theme';

/**
 * Colonne che si spostano e si allargano trascinando i titoli (DX Cluster, Decodifiche e Log del contest): ordine e larghezze restano
 * su questo dispositivo, ognuna con la sua chiave. Le larghezze sono in em, cosi' seguono la dimensione del carattere scelta nelle
 * Impostazioni.
 */
export interface ColumnLayout<C extends string> {
  order: C[];
  /** Larghezze scelte a mano; quelle che mancano tornano di partenza. La colonna elastica senza larghezza prende lo spazio che avanza. */
  widths: Partial<Record<C, number>>;
}

export const COL_MIN_EM = 2.5;
export const COL_MAX_EM = 48;

/** Larghezza della colonna in em, entro i limiti. */
export const clampColumnEm = (v: number) => Math.min(COL_MAX_EM, Math.max(COL_MIN_EM, v));

export const defaultColumnLayout = <C extends string>(columns: readonly C[]): ColumnLayout<C> => ({ order: [...columns], widths: {} });

/** L'ordine e le larghezze salvati sotto `key`; colonne sconosciute o doppie si scartano, quelle nuove entrano al loro posto di partenza. */
export function readColumnLayout<C extends string>(key: string, columns: readonly C[]): ColumnLayout<C> {
  try {
    const raw = JSON.parse(browserStorage()?.getItem(key) ?? 'null') as { order?: unknown; widths?: unknown } | null;
    if (!raw || typeof raw !== 'object') return defaultColumnLayout(columns);
    const known = columns as readonly unknown[];
    const saved = Array.isArray(raw.order) ? raw.order.filter((c, i, all): c is C => known.includes(c) && all.indexOf(c) === i) : [];
    // una colonna nuova (non ancora nell'ordine salvato) entra al suo posto di partenza
    const order = [...saved];
    columns.forEach((c, i) => {
      if (!order.includes(c)) order.splice(Math.min(i, order.length), 0, c);
    });
    const widths: Partial<Record<C, number>> = {};
    if (raw.widths && typeof raw.widths === 'object') {
      for (const c of columns) {
        const v = (raw.widths as Record<string, unknown>)[c];
        if (typeof v === 'number' && Number.isFinite(v)) widths[c] = clampColumnEm(v);
      }
    }
    return { order, widths };
  } catch {
    return defaultColumnLayout(columns);
  }
}

export function saveColumnLayout<C extends string>(key: string, layout: ColumnLayout<C>): void {
  try {
    browserStorage()?.setItem(key, JSON.stringify(layout));
  } catch {
    // senza archivio la scelta vale finche' la pagina e' aperta
  }
}

/** Sposta `col` subito prima di `before` (in fondo con `null`). */
export function placeColumn<C extends string>(order: readonly C[], col: C, before: C | null): C[] {
  if (before === col) return [...order];
  const rest = order.filter((c) => c !== col);
  const at = before === null ? rest.length : rest.indexOf(before);
  rest.splice(at < 0 ? rest.length : at, 0, col);
  return rest;
}
