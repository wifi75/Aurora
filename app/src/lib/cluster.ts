import type { ClusterSpot } from '../api/client';

export interface SpotFilters {
  /** `all` oppure `now` (la banda su cui e' JTDX). */
  band: string;
  /** `all` oppure un modo (FT8, FT4, CW, SSB, RTTY). */
  mode: string;
  search: string;
}

/** Gli spot che passano i filtri della vista (l'ordine non cambia). */
export function filterSpots(spots: readonly ClusterSpot[], filters: SpotFilters, currentBand: string | null): ClusterSpot[] {
  const search = filters.search.trim().toUpperCase();
  return spots.filter((s) => {
    if (filters.band === 'now' && s.band !== currentBand) return false;
    if (filters.mode !== 'all' && s.mode !== filters.mode) return false;
    if (search !== '' && !s.dx_call.includes(search)) return false;
    return true;
  });
}

/** La frequenza come sui cluster italiani: kHz con il punto delle migliaia e la virgola dei decimi (14074 -> "14.074,00"). */
export function formatKhz(khz: number): string {
  const [whole = '', frac = '00'] = khz.toFixed(2).split('.');
  return `${whole.replace(/\B(?=(\d{3})+(?!\d))/g, '.')},${frac}`;
}

/** Secondi da quando Aurora ha ricevuto lo spot. */
export function spotAge(spot: ClusterSpot, now = Date.now()): number {
  return Math.max(0, Math.round((now - spot.received_ms) / 1000));
}
