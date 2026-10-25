import { create } from 'zustand';
import { browserStorage } from './theme';

/*
 * Preferenze generali degli avvisi sonori («Avvisi sonori», volume, «non suonare mentre trasmetti»).
 * Quale suono parte per ogni evento si sceglie in `soundAssign.ts`; i suoni stanno in `soundBank.ts`.
 */

/** I quattro suoni che c'erano prima della scelta per evento: servono solo a migrare la vecchia preferenza «suono di avviso». */
export const SOUND_NAMES = ['chime', 'bell', 'pip', 'horn'] as const;
export type SoundName = (typeof SOUND_NAMES)[number];

export interface AlertPrefs {
  enabled: boolean;
  /** Da 0 a 1: il volume generale di tutti i suoni. */
  volume: number;
  /** Il vecchio suono unico degli avvisi «Play alert sound»: si usa solo per migrare verso l'assegnazione per evento. */
  sound: SoundName;
  /** Non suonare mentre si trasmette (di base si': un suono in cuffia o nel microfono durante il Tx disturba). */
  muteWhileTx: boolean;
}

export const ALERTS_KEY = 'aurora-alerts';
const DEFAULTS: AlertPrefs = { enabled: false, volume: 0.5, sound: 'chime', muteWhileTx: true };

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

/** Gli avvisi partono spenti: un suono mai richiesto in una sala radio e' peggio di nessun suono. */
export function readAlerts(storage: ReadableStorage): AlertPrefs {
  try {
    const raw = storage?.getItem(ALERTS_KEY);
    if (!raw) return { ...DEFAULTS };
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== 'object' || parsed === null) return { ...DEFAULTS };
    const { enabled, volume, sound, muteWhileTx } = parsed as Record<string, unknown>;
    return {
      enabled: enabled === true,
      volume: typeof volume === 'number' && Number.isFinite(volume) ? Math.min(1, Math.max(0, volume)) : DEFAULTS.volume,
      sound: SOUND_NAMES.find((n) => n === sound) ?? DEFAULTS.sound,
      muteWhileTx: typeof muteWhileTx === 'boolean' ? muteWhileTx : DEFAULTS.muteWhileTx,
    };
  } catch {
    return { ...DEFAULTS };
  }
}

export function saveAlerts(storage: WritableStorage, prefs: AlertPrefs): void {
  try {
    storage?.setItem(ALERTS_KEY, JSON.stringify(prefs));
  } catch {
    // senza archivio la scelta non viene ricordata, ma gli avvisi funzionano lo stesso
  }
}

interface AlertState {
  prefs: AlertPrefs;
  set(next: AlertPrefs): void;
}

/** Le preferenze condivise da tutta l'interfaccia: la barra in alto e la scheda Notifiche restano allineate. */
export const useAlertPrefs = create<AlertState>((set) => ({
  prefs: readAlerts(browserStorage()),
  set(next) {
    saveAlerts(browserStorage(), next);
    set({ prefs: next });
  },
}));
