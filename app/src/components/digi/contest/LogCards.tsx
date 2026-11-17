import type { ReactNode } from 'react';

/**
 * Le schede del Log e del DX Cluster: il contenuto e' lo stesso pannello della pagina (`DigiLog` con `view='log'` o `view='cluster'`),
 * qui messo in una scheda spostabile. Il pannello riempie la scheda.
 */
export function LogCard({ log }: { log: ReactNode }) {
  return <div className="ctw-fill ctw-fill--log">{log}</div>;
}

export function ClusterCard({ cluster }: { cluster: ReactNode }) {
  return <div className="ctw-fill ctw-fill--cluster">{cluster}</div>;
}
