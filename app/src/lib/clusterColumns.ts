import { browserStorage } from './theme';
import { CLUSTER_COLUMNS, type ClusterColumn } from './clusterOptions';

/**
 * Ordine e larghezze delle colonne del DX Cluster scelti trascinando i titoli (come nel log delle pagine PSK, RTTY e CW): restano su
 * questo dispositivo. Le larghezze sono in em, cosi' seguono la dimensione del carattere scelta nelle Impostazioni.
 */
export interface ClusterColumnLayout {
  order: ClusterColumn[];
  /** Larghezze scelte a mano; quelle che mancano tornano di partenza. Commenti senza larghezza prende lo spazio che avanza. */
  widths: Partial<Record<ClusterColumn, number>>;
}

/** Larghezza di partenza di ogni colonna, in em (Commenti: il minimo quando prende lo spazio che avanza). */
export const CLUSTER_COL_EM: Record<ClusterColumn, number> = { de: 5.6, freq: 5.6, dx: 7, country: 13, mode: 4.4, comment: 9, time: 4.8 };
export const CLUSTER_COL_MIN_EM = 2.5;
export const CLUSTER_COL_MAX_EM = 48;

const KEY = 'aurora.cluster.cols';

export const defaultClusterLayout = (): ClusterColumnLayout => ({ order: [...CLUSTER_COLUMNS], widths: {} });

/** Larghezza della colonna in em, entro i limiti. */
export const clampColumnEm = (v: number) => Math.min(CLUSTER_COL_MAX_EM, Math.max(CLUSTER_COL_MIN_EM, v));

export function readClusterLayout(): ClusterColumnLayout {
  try {
    const raw = JSON.parse(browserStorage()?.getItem(KEY) ?? 'null') as { order?: unknown; widths?: unknown } | null;
    if (!raw || typeof raw !== 'object') return defaultClusterLayout();
    const saved = Array.isArray(raw.order) ? raw.order.filter((c, i, all): c is ClusterColumn => (CLUSTER_COLUMNS as readonly unknown[]).includes(c) && all.indexOf(c) === i) : [];
    // una colonna nuova (non ancora nell'ordine salvato) entra al suo posto di partenza
    const order = [...saved];
    CLUSTER_COLUMNS.forEach((c, i) => {
      if (!order.includes(c)) order.splice(Math.min(i, order.length), 0, c);
    });
    const widths: Partial<Record<ClusterColumn, number>> = {};
    if (raw.widths && typeof raw.widths === 'object') {
      for (const c of CLUSTER_COLUMNS) {
        const v = (raw.widths as Record<string, unknown>)[c];
        if (typeof v === 'number' && Number.isFinite(v)) widths[c] = clampColumnEm(v);
      }
    }
    return { order, widths };
  } catch {
    return defaultClusterLayout();
  }
}

export function saveClusterLayout(layout: ClusterColumnLayout): void {
  try {
    browserStorage()?.setItem(KEY, JSON.stringify(layout));
  } catch {
    // senza archivio la scelta vale finche' la pagina e' aperta
  }
}

/** Sposta `col` subito prima di `before` (in fondo con `null`). */
export function placeColumn(order: readonly ClusterColumn[], col: ClusterColumn, before: ClusterColumn | null): ClusterColumn[] {
  if (before === col) return [...order];
  const rest = order.filter((c) => c !== col);
  const at = before === null ? rest.length : rest.indexOf(before);
  rest.splice(at < 0 ? rest.length : at, 0, col);
  return rest;
}
