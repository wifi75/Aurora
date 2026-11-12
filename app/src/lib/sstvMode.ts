import { create } from 'zustand';

/**
 * Il modo SSTV della ricezione (quello scelto per «Ricevi in…» o quello dell'ultima immagine in arrivo): il compositore lo segue, cosi'
 * «Trasmetti» risponde nello stesso modo in cui si sta ricevendo. Resta modificabile a mano nel compositore prima di trasmettere.
 */
interface SstvModeState {
  rxMode: string | null;
  setRxMode(mode: string | null): void;
}

export const useSstvMode = create<SstvModeState>((set) => ({
  rxMode: null,
  setRxMode: (mode) => set((s) => (mode && mode !== s.rxMode ? { rxMode: mode } : s)),
}));
