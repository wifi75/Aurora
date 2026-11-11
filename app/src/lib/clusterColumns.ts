import { CLUSTER_COLUMNS, type ClusterColumn } from './clusterOptions';
import { COL_MAX_EM, COL_MIN_EM, defaultColumnLayout, placeColumn as placeAny, readColumnLayout, saveColumnLayout, type ColumnLayout } from './movableColumns';

export { clampColumnEm } from './movableColumns';

/**
 * Ordine e larghezze delle colonne del DX Cluster scelti trascinando i titoli (come nel log delle pagine PSK, RTTY e CW): restano su
 * questo dispositivo. Il meccanismo e' quello comune di `movableColumns`; qui restano la chiave e le larghezze di partenza del cluster.
 */
export type ClusterColumnLayout = ColumnLayout<ClusterColumn>;

/** Larghezza di partenza di ogni colonna, in em (Commenti: il minimo quando prende lo spazio che avanza). */
export const CLUSTER_COL_EM: Record<ClusterColumn, number> = { de: 5.6, freq: 5.6, dx: 7, country: 13, mode: 4.4, comment: 9, time: 4.8 };
export const CLUSTER_COL_MIN_EM = COL_MIN_EM;
export const CLUSTER_COL_MAX_EM = COL_MAX_EM;

/** La chiave di sempre: chi aveva gia' sistemato le colonne le ritrova come le aveva lasciate. */
export const CLUSTER_COLS_KEY = 'aurora.cluster.cols';

export const defaultClusterLayout = (): ClusterColumnLayout => defaultColumnLayout(CLUSTER_COLUMNS);
export const readClusterLayout = (): ClusterColumnLayout => readColumnLayout(CLUSTER_COLS_KEY, CLUSTER_COLUMNS);
export const saveClusterLayout = (layout: ClusterColumnLayout): void => saveColumnLayout(CLUSTER_COLS_KEY, layout);

/** Sposta `col` subito prima di `before` (in fondo con `null`). */
export const placeColumn = (order: readonly ClusterColumn[], col: ClusterColumn, before: ClusterColumn | null): ClusterColumn[] => placeAny(order, col, before);
