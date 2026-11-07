import { create } from 'zustand';
import { browserStorage } from './theme';

/**
 * Dimensione del testo: ingrandisce (o rimpicciolisce) tutta l'interfaccia con `zoom` sulla radice, cosi' testo, pulsanti, elenchi e
 * cascata crescono insieme e niente si sovrappone. Si sceglie in Impostazioni > Generali o con Ctrl +, Ctrl - e Ctrl 0 (come nei browser).
 * Resta su questo dispositivo: il telefono e il PC hanno schermi diversi.
 */
export const UI_SCALES = [0.9, 1, 1.1, 1.25, 1.4, 1.6] as const;
const KEY = 'aurora-ui-scale';
const MIN = 0.9;
const MAX = 1.6;

function clamp(v: number): number {
  return Math.min(MAX, Math.max(MIN, Math.round(v * 100) / 100));
}

export function readScale(): number {
  try {
    const v = Number(browserStorage()?.getItem(KEY));
    return Number.isFinite(v) && v > 0 ? clamp(v) : 1;
  } catch {
    return 1;
  }
}

function apply(scale: number): void {
  document.documentElement.style.setProperty('zoom', scale === 1 ? '' : String(scale));
}

interface ScaleState {
  scale: number;
  set(scale: number): void;
  step(dir: 1 | -1): void;
}

export const useUiScale = create<ScaleState>((set, get) => ({
  scale: readScale(),
  set(scale) {
    const next = clamp(scale);
    try {
      browserStorage()?.setItem(KEY, String(next));
    } catch {
      // senza archivio la scelta vale finche' resta aperta la finestra
    }
    apply(next);
    set({ scale: next });
  },
  step(dir) {
    const now = get().scale;
    const next = dir > 0 ? UI_SCALES.find((s) => s > now + 0.001) : [...UI_SCALES].reverse().find((s) => s < now - 0.001);
    if (next !== undefined) get().set(next);
  },
}));

/** All'avvio: applica la scelta salvata e ascolta Ctrl +, Ctrl - e Ctrl 0 (anche con il tastierino). */
export function startUiScale(): void {
  apply(useUiScale.getState().scale);
  window.addEventListener('keydown', (e) => {
    if (!(e.ctrlKey || e.metaKey) || e.altKey) return;
    const k = e.key;
    if (k === '+' || k === '=' || e.code === 'NumpadAdd') useUiScale.getState().step(1);
    else if (k === '-' || k === '_' || e.code === 'NumpadSubtract') useUiScale.getState().step(-1);
    else if (k === '0' || e.code === 'Numpad0') useUiScale.getState().set(1);
    else return;
    e.preventDefault();
  });
  // gli altri dispositivi e le finestre staccate cambiano insieme
  window.addEventListener('storage', (e) => {
    if (e.key === KEY) {
      const v = readScale();
      apply(v);
      useUiScale.setState({ scale: v });
    }
  });
}
