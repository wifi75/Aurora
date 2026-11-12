import type { ClusterColumn } from './clusterOptions';

/**
 * Ordine e larghezze delle colonne del DX Cluster scelti trascinando i titoli (come nel log delle pagine PSK, RTTY e CW): restano su
 * questo dispositivo. Il meccanismo e' quello comune di `movableColumns`; qui restano la chiave e le larghezze di partenza del cluster.
 */

/** Larghezza di partenza di ogni colonna, in em (Commenti: il minimo quando prende lo spazio che avanza). */
export const CLUSTER_COL_EM: Record<ClusterColumn, number> = { de: 5.6, freq: 5.6, dx: 7, country: 13, mode: 4.4, comment: 9, time: 4.8 };

/** La chiave di sempre: chi aveva gia' sistemato le colonne le ritrova come le aveva lasciate. */
export const CLUSTER_COLS_KEY = 'aurora.cluster.cols';
