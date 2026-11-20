/*
 * L'etichetta «satellite in arrivo» della barra in alto: logica pura e piccolo archivio condiviso.
 * Compare per i satelliti seguiti (stessa regola degli avvisi della pagina Satelliti) da `leadMin` minuti prima del sorgere,
 * con il conto alla rovescia; dopo l'AOS dice quanto manca alla LOS e sparisce da sola dopo `SHOW_AFTER_AOS_S` secondi.
 * Si chiude a mano, ma la chiusura vale solo per quel passaggio di quel satellite.
 *
 * L'inizio di un passaggio gia' in corso, nell'elenco dei passaggi, e' l'istante in cui lo si e' chiesto (cambia a ogni
 * aggiornamento): per questo l'AOS vero si ricorda da quando il passaggio era ancora in arrivo. Aprendo Aurora a passaggio
 * gia' iniziato l'etichetta non compare.
 */

import { useSyncExternalStore } from 'react';
import { MATCH_S, isWatched, type AlertPass, type SatAlertSettings } from './satAlerts';

/** Dopo l'AOS l'etichetta resta ancora per questi secondi, poi sparisce da sola. */
export const SHOW_AFTER_AOS_S = 60;

export interface PassRef {
  norad: number;
  /** Fine del passaggio (stabile fra un aggiornamento dell'elenco e l'altro). */
  los: number;
}

export interface KnownPass extends PassRef {
  /** Inizio previsto, ricordato finche' il passaggio era ancora in arrivo. */
  aos: number;
}

export interface SatLabel extends PassRef {
  name: string;
  /** `soon`: manca `seconds` al sorgere; `now`: in vista, manca `seconds` al tramonto. */
  phase: 'soon' | 'now';
  seconds: number;
}

const same = (a: PassRef, b: PassRef) => a.norad === b.norad && Math.abs(a.los - b.los) <= MATCH_S;
const KEEP_S = 3600;

/** Ricorda l'inizio dei passaggi ancora in arrivo e butta quelli finiti da piu' di un'ora. */
export function rememberPasses(known: readonly KnownPass[], passes: readonly AlertPass[], now: number): KnownPass[] {
  const out = known.filter((k) => k.los > now - KEEP_S);
  for (const p of passes) {
    if (p.aos_unix <= now) continue;
    const i = out.findIndex((k) => same(k, { norad: p.norad, los: p.los_unix }));
    const rec: KnownPass = { norad: p.norad, los: p.los_unix, aos: p.aos_unix };
    if (i >= 0) out[i] = rec;
    else out.push(rec);
  }
  return out;
}

export interface LabelInput {
  passes: readonly AlertPass[];
  known: readonly KnownPass[];
  dismissed: readonly PassRef[];
  now: number;
  settings: SatAlertSettings;
  selected: number | null;
  tracked: readonly number[];
  minEl: number;
}

/** Cosa mostrare adesso nella barra, o `null`. Fra piu' passaggi vince quello che inizia prima. */
export function satLabel(input: LabelInput): SatLabel | null {
  const { passes, known, dismissed, now, settings, selected, tracked, minEl } = input;
  if (!settings.enabled || settings.leadMin <= 0) return null;
  let best: (SatLabel & { aos: number }) | null = null;
  for (const p of passes) {
    if (!isWatched(p.norad, settings, selected, tracked)) continue;
    if (p.max_el_deg < minEl || now >= p.los_unix) continue;
    const ref = { norad: p.norad, los: p.los_unix };
    if (dismissed.some((d) => same(d, ref))) continue;
    let cand: (SatLabel & { aos: number }) | null = null;
    if (p.aos_unix > now) {
      if (now >= p.aos_unix - settings.leadMin * 60) cand = { ...ref, name: p.name, phase: 'soon', seconds: Math.ceil(p.aos_unix - now), aos: p.aos_unix };
    } else {
      const k = known.find((x) => same(x, ref));
      if (k && now >= k.aos && now < k.aos + SHOW_AFTER_AOS_S) cand = { ...ref, name: p.name, phase: 'now', seconds: Math.ceil(p.los_unix - now), aos: k.aos };
    }
    if (cand && (!best || cand.aos < best.aos)) best = cand;
  }
  if (!best) return null;
  const { aos: _aos, ...label } = best;
  return label;
}

// ---- archivio condiviso fra il motore alla radice dell'app e l'etichetta ----

interface State {
  label: SatLabel | null;
  dismissed: readonly PassRef[];
}

let state: State = { label: null, dismissed: [] };
const listeners = new Set<() => void>();
const emit = () => listeners.forEach((l) => l());

const sameLabel = (a: SatLabel | null, b: SatLabel | null) => a === b || (!!a && !!b && a.norad === b.norad && a.los === b.los && a.phase === b.phase && a.seconds === b.seconds && a.name === b.name);

export const satLabelStore = {
  subscribe(l: () => void) {
    listeners.add(l);
    return () => {
      listeners.delete(l);
    };
  },
  get: () => state,
  setLabel(label: SatLabel | null) {
    if (sameLabel(state.label, label)) return;
    state = { ...state, label };
    emit();
  },
  /** La ✕: chiude l'avviso di questo passaggio di questo satellite. */
  dismiss(ref: PassRef) {
    state = { label: null, dismissed: [...state.dismissed, { norad: ref.norad, los: ref.los }].slice(-30) };
    emit();
  },
  reset() {
    state = { label: null, dismissed: [] };
    emit();
  },
};

export function useSatLabel(): State {
  return useSyncExternalStore(satLabelStore.subscribe, satLabelStore.get, satLabelStore.get);
}
