import { create } from 'zustand';

/** I controlli a valore numerico del motore (oggi la deriva massima di Q65, 0-50), letti insieme agli interruttori. */
interface ControlValues {
  values: Record<string, number>;
  set: (values: Record<string, number>) => void;
}

export const useControlValues = create<ControlValues>((set) => ({
  values: {},
  set: (values) => set({ values }),
}));

/** Tiene solo le voci numeriche della risposta; il resto si scarta. */
export function parseControlValues(raw: unknown): Record<string, number> {
  if (!raw || typeof raw !== 'object') return {};
  const out: Record<string, number> = {};
  for (const [id, v] of Object.entries(raw as Record<string, unknown>)) {
    if (typeof v === 'number' && Number.isFinite(v)) out[id] = v;
  }
  return out;
}
