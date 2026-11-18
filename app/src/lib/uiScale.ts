import { create } from 'zustand';
import { browserStorage } from './theme';

/**
 * Dimensione del testo: ingrandisce (o rimpicciolisce) tutta l'interfaccia con `zoom` sulla radice, cosi' testo, pulsanti, elenchi e
 * cascata crescono insieme e niente si sovrappone. Si sceglie in Impostazioni > Generali o con Ctrl +, Ctrl - e Ctrl 0 (come nei browser).
 * Resta su questo dispositivo: il telefono e il PC hanno schermi diversi.
 */
export const UI_SCALES = [0.6, 0.7, 0.8, 0.9, 1, 1.1, 1.25, 1.4, 1.6] as const;
const KEY = 'aurora-ui-scale';
const MIN = 0.6;
const MAX = 1.6;

/**
 * La dimensione adatta alla larghezza dello schermo: le pagine sono pensate per circa 1800 punti. Su un portatile da 1366 punti (o con la scala di
 * Windows al 125%, che li riduce a circa 1100) tutto uscirebbe dai bordi: si parte piu' piccoli, e chi vuole cambia con Ctrl + e Ctrl -.
 */
export function autoScale(width: number): number {
  if (!Number.isFinite(width) || width <= 0) return 1;
  if (width < 1100) return 0.65;
  if (width < 1250) return 0.75;
  if (width < 1400) return 0.8;
  if (width < 1500) return 0.85;
  if (width < 1700) return 0.9;
  return 1;
}

/** L'utente ha scelto una dimensione (se no vale quella automatica, che segue lo schermo). */
export function hasChosenScale(): boolean {
  try {
    return browserStorage()?.getItem(KEY) != null;
  } catch {
    return false;
  }
}

function clamp(v: number): number {
  return Math.min(MAX, Math.max(MIN, Math.round(v * 100) / 100));
}

export function readScale(): number {
  const auto = clamp(autoScale(typeof window === 'undefined' ? 0 : window.innerWidth));
  try {
    const raw = browserStorage()?.getItem(KEY);
    if (raw == null) return auto;
    const v = Number(raw);
    return Number.isFinite(v) && v > 0 ? clamp(v) : auto;
  } catch {
    return auto;
  }
}

function apply(scale: number): void {
  document.documentElement.style.setProperty('zoom', scale === 1 ? '' : String(scale));
  // le unita' vh/dvh non seguono lo zoom: le pagine che riempiono lo schermo dividono per questo valore
  document.documentElement.style.setProperty('--ui-zoom', String(scale));
}

interface ScaleState {
  scale: number;
  set(scale: number): void;
  step(dir: 1 | -1): void;
  /** Torna alla dimensione adatta allo schermo (cancella la scelta fatta). */
  fit(): void;
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
  fit() {
    try {
      browserStorage()?.removeItem(KEY);
    } catch {
      // senza archivio la dimensione automatica vale comunque
    }
    const next = clamp(autoScale(window.innerWidth));
    apply(next);
    set({ scale: next });
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
