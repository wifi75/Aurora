import type { ClusterSpot } from '../api/client';

export interface SpotFilters {
  /** Bande scelte: `now` e' la banda su cui e' il motore; vuoto = tutte. */
  bands: string[];
  /** Modi scelti (FT8, FT4, CW, SSB, RTTY...); vuoto = tutti. */
  modes: string[];
  search: string;
}

/** Gli spot che passano i filtri della vista (l'ordine non cambia). */
export function filterSpots(spots: readonly ClusterSpot[], filters: SpotFilters, currentBand: string | null): ClusterSpot[] {
  const search = filters.search.trim().toUpperCase();
  return spots.filter((s) => {
    if (filters.bands.length > 0) {
      const mine = filters.bands.includes('now') && s.band === currentBand;
      if (!mine && !(s.band !== null && filters.bands.includes(s.band))) return false;
    }
    if (filters.modes.length > 0 && !(s.mode !== null && filters.modes.includes(s.mode))) return false;
    if (search !== '' && !s.dx_call.includes(search)) return false;
    return true;
  });
}

/** Il filtro salvato dalle versioni precedenti era una sola voce (`all`, `now` o una banda): lo porta alla lista. */
export function toList(stored: unknown): string[] {
  if (Array.isArray(stored)) return stored.filter((v): v is string => typeof v === 'string');
  if (typeof stored === 'string' && stored !== 'all' && stored !== '') return [stored];
  return [];
}

/** Secondi da quando Aurora ha ricevuto lo spot. */
export function spotAge(spot: ClusterSpot, now = Date.now()): number {
  return Math.max(0, Math.round((now - spot.received_ms) / 1000));
}
