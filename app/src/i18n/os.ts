import { create } from 'zustand';

interface EngineOsState {
  /** Il sistema del PC dove gira il motore di Aurora, come lo da' `/health` (`windows`, `linux`, `macos`). */
  os: string;
  setOs(os: string): void;
}

/**
 * Il sistema su cui gira il motore (non quello del browser: dal telefono si comanda un PC). Finche' non si sa vale Windows, com'era prima.
 * Lo usa la traduzione: i testi che nominano Windows hanno una variante `.sys` per gli altri sistemi (`strings/system.ts`).
 */
export const useEngineOs = create<EngineOsState>((set) => ({
  os: 'windows',
  setOs(os) {
    const next = os.trim().toLowerCase();
    if (next !== '') set({ os: next });
  },
}));

export function isWindowsOs(os: string): boolean {
  return os === 'windows';
}
