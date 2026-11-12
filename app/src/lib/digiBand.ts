import type { DigiSignal } from '../api/schemas';
import { onDigi } from './digiBus';

/** I segnali di tutta la banda (evento `band` dei modi testuali): restano anche lasciando la pagina. */
let signals: DigiSignal[] = [];
let mode = '';
const listeners = new Set<() => void>();

onDigi((ev) => {
  if (ev.kind !== 'band') return;
  signals = ev.signals ?? [];
  mode = ev.mode ?? '';
  for (const l of listeners) l();
});

export function getBandSignals(): DigiSignal[] {
  return signals;
}

/** Il modo dei segnali (`cw`, `psk31`, `psk63`), vuoto prima del primo elenco. */
export function getBandMode(): string {
  return mode;
}

/** «Pulisci»: l'elenco si svuota subito, senza aspettare l'elenco vuoto del core. */
export function clearBandSignals(): void {
  signals = [];
  for (const l of listeners) l();
}

export function onBandSignals(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Il nominativo da mostrare di un segnale: chi chiama CQ, poi chi trasmette (DE), poi il piu' probabile. */
export function mainCall(s: DigiSignal): DigiSignal['calls'][number] | null {
  return s.calls.find((c) => c.role === 'cq') ?? s.calls.find((c) => c.role === 'de') ?? s.calls[0] ?? null;
}

/** Il testo diviso in pezzi sicuri e incerti (in grigio), nell'ordine. */
export function splitUnsure(text: string, unsure: number[]): { text: string; unsure: boolean }[] {
  const set = new Set(unsure);
  const out: { text: string; unsure: boolean }[] = [];
  Array.from(text).forEach((ch, i) => {
    const u = set.has(i) && ch.trim() !== '';
    const last = out[out.length - 1];
    if (last && last.unsure === u) last.text += ch;
    else out.push({ text: ch, unsure: u });
  });
  return out;
}

/** Un nominativo scritto sulla cascata sopra la traccia del suo segnale. */
export interface WaterfallLabel {
  hz: number;
  text: string;
  cq: boolean;
  sure: boolean;
}

/**
 * Le etichette dei nominativi sulla cascata: un segnale ha la sua finche' il nominativo si sente. Quelli non piu' sentiti da oltre 75 s
 * (`stale`, lo dice il core) restano nell'elenco dei segnali ma non sulla cascata.
 */
export function waterfallLabels(list: DigiSignal[]): WaterfallLabel[] {
  return list.flatMap((sig) => {
    const c = mainCall({ ...sig, calls: sig.calls.filter((x) => x.stale !== true) });
    return c ? [{ hz: sig.freq_hz, text: c.call, cq: c.role === 'cq', sure: c.verified }] : [];
  });
}
