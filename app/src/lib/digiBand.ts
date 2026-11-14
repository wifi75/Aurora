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

/** Una stazione del Radar: un nominativo verificato, una riga sola anche se piu' canali lo hanno letto. */
export interface RadarStation {
  call: string;
  freqHz: number;
  wpm: number | null;
  snrDb: number;
  cq: boolean;
  country: string | null;
  /** Non la si sente da oltre 75 s. */
  stale: boolean;
  /** Indice del colore della stazione (0..RADAR_COLORS-1), fisso per nominativo. */
  color: number;
}

/** Quanti colori distinti hanno le stazioni del Radar (le classi `radar-c0`...). */
export const RADAR_COLORS = 10;

/** Il colore di un nominativo: sempre lo stesso per la stessa stazione. */
export function radarColor(call: string): number {
  let h = 2166136261;
  for (let i = 0; i < call.length; i++) {
    h ^= call.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return (h >>> 0) % RADAR_COLORS;
}

/**
 * Le stazioni del Radar: solo i nominativi verificati (sentiti piu' volte con caratteri sicuri) di chi trasmette (CQ o DE), una riga per
 * nominativo. Se due canali
 * leggono lo stesso nominativo resta quello sentito di recente e piu' forte. Ordine: dalla frequenza piu' bassa.
 */
export function radarStations(list: DigiSignal[]): RadarStation[] {
  const best = new Map<string, RadarStation>();
  for (const sig of list) {
    for (const c of sig.calls) {
      // solo chi trasmette (dopo CQ o DE): i nominativi che una stazione nomina («IV3IPS 599 DE SV5BYP») non sono stazioni della banda
      if (!c.verified || c.role === 'other') continue;
      const st: RadarStation = {
        call: c.call,
        freqHz: sig.freq_hz,
        wpm: sig.wpm ?? null,
        snrDb: sig.snr_db,
        cq: c.role === 'cq' || (sig.cq && c.role !== 'other'),
        country: c.country ?? null,
        stale: c.stale === true,
        color: radarColor(c.call),
      };
      const old = best.get(c.call);
      if (!old || (old.stale && !st.stale) || (old.stale === st.stale && st.snrDb > old.snrDb)) best.set(c.call, st);
    }
  }
  return [...best.values()].sort((a, b) => a.freqHz - b.freqHz);
}
