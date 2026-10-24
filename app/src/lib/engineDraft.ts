import { create } from 'zustand';

/** Le modifiche alle impostazioni del motore non ancora applicate (valori di testo, come nel file), condivise tra le schede. */
interface DraftState {
  changes: Record<string, string>;
  /** Imposta un valore; se torna uguale a quello attuale la modifica sparisce. */
  set(key: string, value: string, current: string | undefined): void;
  clear(): void;
}

export const useEngineDraft = create<DraftState>((set) => ({
  changes: {},
  set(key, value, current) {
    set((s) => {
      const changes = { ...s.changes };
      if (value === (current ?? '')) delete changes[key];
      else changes[key] = value;
      return { changes };
    });
  },
  clear() {
    set({ changes: {} });
  },
}));
