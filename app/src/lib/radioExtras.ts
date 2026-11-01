import { create } from 'zustand';
import { browserStorage } from './theme';

/** Le voci del display esteso, ognuna accendibile dalle impostazioni. */
export const RADIO_EXTRA_KEYS = ['vfoB', 'meters', 'supply', 'frontEnd', 'agc', 'noise', 'gains', 'ritXit', 'tuner', 'vfo'] as const;
export type RadioExtraKey = (typeof RADIO_EXTRA_KEYS)[number];
export type RadioExtraPrefs = Record<RadioExtraKey, boolean>;

const KEY = 'aurora-radio-extras';

/** Di base: secondo VFO, strumenti e filtri principali; il resto si accende a scelta. */
export function defaultRadioExtras(): RadioExtraPrefs {
  return { vfoB: true, meters: true, supply: false, frontEnd: true, agc: true, noise: true, gains: false, ritXit: false, tuner: false, vfo: false };
}

function read(): RadioExtraPrefs {
  const out = defaultRadioExtras();
  try {
    const raw = browserStorage()?.getItem(KEY);
    const parsed = raw ? (JSON.parse(raw) as Partial<Record<string, unknown>>) : {};
    for (const k of RADIO_EXTRA_KEYS) if (typeof parsed[k] === 'boolean') out[k] = parsed[k] as boolean;
  } catch {
    // preferenze illeggibili: si resta su quelle di base
  }
  return out;
}

export const useRadioExtras = create<{ prefs: RadioExtraPrefs; set(patch: Partial<RadioExtraPrefs>): void }>((set, get) => ({
  prefs: read(),
  set(patch) {
    const prefs = { ...get().prefs, ...patch };
    try {
      browserStorage()?.setItem(KEY, JSON.stringify(prefs));
    } catch {
      // senza memoria del browser vale fino alla chiusura
    }
    set({ prefs });
  },
}));

/** «14074000» → «14.074.00» come sul display della radio. */
export function formatVfo(hz: number): string {
  const khz = Math.floor(hz / 10) / 100;
  const [int = '0', dec = '00'] = khz.toFixed(2).split('.');
  const mhz = int.length > 3 ? `${int.slice(0, -3)}.${int.slice(-3)}` : int;
  return `${mhz}.${dec}`;
}

/** Il nome dell'AGC come lo numera Hamlib. */
export function agcName(code: number): string {
  return ({ 0: 'OFF', 1: 'SFAST', 2: 'FAST', 3: 'SLOW', 4: 'USER', 5: 'MED', 6: 'AUTO' } as Record<number, string>)[code] ?? String(code);
}
