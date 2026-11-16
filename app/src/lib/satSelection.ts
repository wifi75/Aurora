/** Scelta dei satelliti: la casella (tracked) e' la verita'; il satellite scelto (selected) deve sempre essere spuntato. */
export interface SatSelectionState {
  selected: number | null;
  tracked: readonly number[];
}

/** Il satellite scelto che vale davvero: se non e' spuntato diventa il primo spuntato; solo senza spunte non c'e' nessuno. */
export function normalizeSatSelection(s: SatSelectionState): { selected: number | null; changed: boolean } {
  const selected = s.selected !== null && s.tracked.includes(s.selected) ? s.selected : (s.tracked[0] ?? null);
  return { selected, changed: selected !== s.selected };
}

/** Clic sul nome (o sulla mappa): lo sceglie e lo spunta se manca. */
export function pickSatellite(s: SatSelectionState, norad: number): { selected: number; tracked: number[] } {
  return { selected: norad, tracked: s.tracked.includes(norad) ? [...s.tracked] : [...s.tracked, norad] };
}

/**
 * Casella spuntata o tolta. Spuntando senza una scelta, quel satellite diventa anche il scelto;
 * togliendo la spunta al satellite scelto, la scelta passa al prossimo spuntato (o a nessuno).
 */
export function setTracked(s: SatSelectionState, norad: number, on: boolean): { selected: number | null; tracked: number[] } {
  const without = s.tracked.filter((n) => n !== norad);
  if (on) {
    const tracked = [...without, norad];
    return { selected: normalizeSatSelection({ selected: s.selected, tracked }).selected === s.selected && s.selected !== null ? s.selected : norad, tracked };
  }
  return { selected: s.selected === norad ? (without[0] ?? null) : s.selected, tracked: without };
}

/** Deseleziona tutti: nessuna spunta e nessuna scelta. */
export function clearSatellites(): { selected: null; tracked: number[] } {
  return { selected: null, tracked: [] };
}

/** L'elenco in due gruppi: prima i satelliti spuntati, poi gli altri, ciascuno nell'ordine ricevuto. */
export function groupListing<T extends { norad: number }>(listing: readonly T[], tracked: readonly number[]): { ticked: T[]; rest: T[] } {
  const ticked: T[] = [];
  const rest: T[] = [];
  for (const x of listing) (tracked.includes(x.norad) ? ticked : rest).push(x);
  return { ticked, rest };
}
