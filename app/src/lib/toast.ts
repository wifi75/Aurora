import { create } from 'zustand';
import { useToastPrefs } from './toastPrefs';

export type ToastKind = 'ok' | 'error' | 'info';

export interface Toast {
  id: number;
  text: string;
  kind: ToastKind;
  /** Durata in ms prima che sparisca da sola. */
  ms: number;
  /** Sta uscendo (animazione di uscita in corso). */
  leaving: boolean;
}

/** Quanto resta una notifica: gli errori un po' di piu', per avere il tempo di leggerli. */
export const TOAST_MS: Record<ToastKind, number> = { ok: 4500, info: 5500, error: 8000 };
/** Durata dell'animazione di uscita (deve coincidere con quella del CSS). */
export const TOAST_LEAVE_MS = 350;
/** Al massimo tante notifiche insieme: le piu' vecchie escono. */
const MAX = 4;

let nextId = 1;

interface ToastState {
  toasts: Toast[];
  push(text: string, kind?: ToastKind): number;
  dismiss(id: number): void;
}

export const useToasts = create<ToastState>((set, get) => ({
  toasts: [],
  push(text, kind = 'ok') {
    const id = nextId++;
    const prefs = useToastPrefs.getState().prefs;
    // un tipo spento non compare (gli errori si vedono sempre)
    if (!prefs.show[kind]) return id;
    const ms = Math.round(prefs.seconds[kind] * 1000) || TOAST_MS[kind];
    set((s) => ({ toasts: [...s.toasts.filter((t) => t.text !== text), { id, text, kind, ms, leaving: false }].slice(-MAX) }));
    setTimeout(() => get().dismiss(id), ms);
    return id;
  },
  dismiss(id) {
    if (!get().toasts.some((t) => t.id === id && !t.leaving)) return;
    set((s) => ({ toasts: s.toasts.map((t) => (t.id === id ? { ...t, leaving: true } : t)) }));
    setTimeout(() => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })), TOAST_LEAVE_MS);
  },
}));

/** Mostra una notifica (da qualunque punto dell'interfaccia). */
export function toast(text: string, kind: ToastKind = 'ok'): number {
  return useToasts.getState().push(text, kind);
}
