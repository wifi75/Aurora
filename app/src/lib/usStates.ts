import { create } from 'zustand';
import { getWorkedMap } from '../api/client';
import type { DecodedMessage } from '../api/schemas';
import { knownCallsign, prefetchCallsigns, readyListeners } from '../components/CallsignTip';
import { callOf } from './filters';

/**
 * «Nuovo stato» e «nuovo stato sulla banda» (stati USA, il WAS): lo stato di una stazione viene da QRZ.com o HamQTH (ricerca dei
 * nominativi), gli stati gia' lavorati dal log di Log4OM2 (colonna `state`). Senza ricerche accese o senza Log4OM non segna nulla.
 */
interface UsStates {
  /** Stato -> bande in cui e' gia' lavorato. */
  worked: Map<string, Set<string>>;
  loaded: boolean;
  /** Cresce a ogni risposta delle ricerche: chi filtra ricalcola. */
  version: number;
}

export const useUsStates = create<UsStates>(() => ({ worked: new Map(), loaded: false, version: 0 }));

let loading = false;
let lastLoad = 0;

/** Ricarica gli stati lavorati (al massimo una volta ogni 5 minuti, o subito se `force`). */
export function loadWorkedStates(force = false): void {
  if (loading || (!force && Date.now() - lastLoad < 5 * 60_000)) return;
  loading = true;
  lastLoad = Date.now();
  getWorkedMap()
    .then((r) => {
      const worked = new Map<string, Set<string>>();
      for (const s of (r as { states?: { state: string; bands: string[] }[] }).states ?? []) worked.set(s.state.toUpperCase(), new Set(s.bands.map((b) => b.toLowerCase())));
      useUsStates.setState({ worked, loaded: true });
    })
    .catch(() => undefined)
    .finally(() => {
      loading = false;
    });
}

readyListeners.add(() => useUsStates.setState((s) => ({ version: s.version + 1 })));

const isUsa = (d: Pick<DecodedMessage, 'country'>) => /^united states/i.test(d.country ?? '');

/** Precarica la ricerca dei nominativi USA in vista, per conoscerne lo stato. */
export function prefetchUsStations(decodes: readonly DecodedMessage[]): void {
  const calls = decodes.filter(isUsa).map((d) => callOf(d.text)).filter((c): c is string => !!c);
  if (calls.length > 0) prefetchCallsigns(calls);
}

/** Lo stato della stazione che trasmette, se si sa (due lettere, es. «TX»). */
export function stateOf(d: Pick<DecodedMessage, 'text' | 'country'>): string | null {
  if (!isUsa(d)) return null;
  const call = callOf(d.text);
  const st = call ? knownCallsign(call)?.person?.state?.trim().toUpperCase() : undefined;
  return st && /^[A-Z]{2}$/.test(st) ? st : null;
}

/** «new_state» (mai lavorato), «new_state_band» (non su questa banda) o null. */
export function stateTag(d: Pick<DecodedMessage, 'text' | 'country'>, band: string | null): 'new_state' | 'new_state_band' | null {
  const { worked, loaded } = useUsStates.getState();
  if (!loaded) return null;
  const st = stateOf(d);
  if (!st) return null;
  const bands = worked.get(st);
  if (!bands) return 'new_state';
  return band && !bands.has(band.toLowerCase()) ? 'new_state_band' : null;
}
