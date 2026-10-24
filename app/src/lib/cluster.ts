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

/** Quanto restano sulla cascata gli spot: oltre, sono vecchi per chiamare qualcuno. */
export const SPOT_MAX_AGE_S = 600;
/** Righe di nominativi sovrapposte al massimo sopra la cascata. */
export const SPOT_ROWS = 3;

export interface WaterfallSpot {
  call: string;
  /** Frequenza audio sulla cascata (Hz): quella dello spot meno la frequenza della radio. */
  hz: number;
  freq_khz: number;
  spotter: string;
  mode: string | null;
  /** In quale riga scrivere il nominativo (0 = la piu' bassa); `null` = niente spazio, resta la tacca. */
  row: number | null;
}

/**
 * Gli spot del cluster che cadono dentro la cascata: la frequenza audio e' quella dello spot meno quella della radio, tra
 * `fMinHz` e `fMaxHz`. Per ogni nominativo vale lo spot piu' recente; i nominativi si dispongono su piu' righe per non coprirsi.
 */
export function spotsOnWaterfall(spots: readonly ClusterSpot[], dialHz: number, fMinHz: number, fMaxHz: number, now = Date.now()): WaterfallSpot[] {
  if (dialHz <= 0) return [];
  const latest = new Map<string, ClusterSpot>();
  for (const s of spots) {
    const hz = Math.round(s.freq_khz * 1000 - dialHz);
    if (hz < fMinHz || hz > fMaxHz || spotAge(s, now) > SPOT_MAX_AGE_S) continue;
    const key = s.dx_call.toUpperCase();
    const seen = latest.get(key);
    if (!seen || s.received_ms > seen.received_ms) latest.set(key, s);
  }
  const placed = [...latest.values()]
    .map((s) => ({ call: s.dx_call, hz: Math.round(s.freq_khz * 1000 - dialHz), freq_khz: s.freq_khz, spotter: s.spotter, mode: s.mode, row: null as number | null }))
    .sort((a, b) => a.hz - b.hz);
  // un nominativo occupa circa 80 Hz piu' 30 Hz a lettera sulla larghezza tipica della cascata
  const rowEnd: number[] = [];
  for (const p of placed) {
    const width = 80 + p.call.length * 30;
    const row = rowEnd.findIndex((end) => p.hz >= end);
    const chosen = row >= 0 ? row : rowEnd.length < SPOT_ROWS ? rowEnd.length : -1;
    if (chosen < 0) continue;
    rowEnd[chosen] = p.hz + width;
    p.row = chosen;
  }
  return placed;
}
