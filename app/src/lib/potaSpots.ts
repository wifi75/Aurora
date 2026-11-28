import { create } from 'zustand';
import { baseCall } from './specialCq';

/** Uno spot POTA di un attivatore, come lo da' il core (`GET /api/v1/online/pota`). */
export interface PotaSpot {
  activator: string;
  reference: string;
  name: string;
  /** In kHz, come negli spot POTA (0 = sconosciuta). */
  freq_khz: number;
  mode: string;
}

/** Gli spot per nominativo: sia com'e' scritto (IU3ABC/P) sia senza barre (IU3ABC). */
export function indexSpots(spots: readonly PotaSpot[]): Record<string, PotaSpot> {
  const out: Record<string, PotaSpot> = {};
  for (const s of spots) {
    const call = s.activator.trim().toUpperCase();
    if (call === '') continue;
    out[call] = s;
    const base = baseCall(call);
    if (!(base in out)) out[base] = s;
  }
  return out;
}

/** Lo spot di chi chiama: prima il nominativo intero, poi quello senza barre. */
export function spotFor(byCall: Record<string, PotaSpot>, call: string, base: string): PotaSpot | null {
  return byCall[call.toUpperCase()] ?? byCall[base.toUpperCase()] ?? null;
}

interface PotaState {
  byCall: Record<string, PotaSpot>;
  set(spots: readonly PotaSpot[]): void;
}

/** Gli spot POTA letti dal core (li porta `PotaFeeder`, se la ricerca del parco e' accesa): la lista delle decodifiche li legge da qui. */
export const usePotaSpots = create<PotaState>((set) => ({
  byCall: {},
  set(spots) {
    set({ byCall: indexSpots(spots) });
  },
}));
