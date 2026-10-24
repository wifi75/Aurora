import { create } from 'zustand';
import { COLOR_KEYS, type ColorKey } from './colors';
import { browserStorage } from './theme';

/** Per ogni tipo di messaggio: evidenziarlo (colore, etichetta) e se far suonare l'avviso (scheda Notifiche di JTDX). */
export interface Notify {
  highlight: Record<ColorKey, boolean>;
  sound: Record<ColorKey, boolean>;
}

export type FlagKind = keyof Notify;

const KEY = 'aurora-notify';
/** Chi suona per impostazione predefinita: le novita' rare, come gli avvisi di prima. */
const SOUND_BY_DEFAULT: readonly ColorKey[] = ['my_call', 'new_dxcc', 'new_dxcc_band', 'new_zone', 'new_zone_band', 'new_itu', 'new_grid', 'new_px'];

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

export function defaultNotify(): Notify {
  const highlight = {} as Record<ColorKey, boolean>;
  const sound = {} as Record<ColorKey, boolean>;
  for (const key of COLOR_KEYS) {
    highlight[key] = true;
    sound[key] = SOUND_BY_DEFAULT.includes(key);
  }
  return { highlight, sound };
}

function merge(target: Record<ColorKey, boolean>, source: unknown): void {
  if (typeof source !== 'object' || source === null) return;
  for (const key of COLOR_KEYS) {
    const value = (source as Record<string, unknown>)[key];
    if (typeof value === 'boolean') target[key] = value;
  }
}

export function readNotify(storage: ReadableStorage): Notify {
  const notify = defaultNotify();
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return notify;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    merge(notify.highlight, parsed.highlight);
    merge(notify.sound, parsed.sound);
  } catch {
    return defaultNotify();
  }
  return notify;
}

export function saveNotify(storage: WritableStorage, notify: Notify): void {
  try {
    storage?.setItem(KEY, JSON.stringify(notify));
  } catch {
    // senza archivio le scelte non vengono ricordate, ma l'interfaccia funziona lo stesso
  }
}

interface NotifyState {
  notify: Notify;
  setFlag(kind: FlagKind, key: ColorKey, value: boolean): void;
  reset(): void;
}

export const useNotify = create<NotifyState>((set, get) => ({
  notify: readNotify(browserStorage()),
  setFlag(kind, key, value) {
    const current = get().notify;
    const next: Notify = { ...current, [kind]: { ...current[kind], [key]: value } };
    saveNotify(browserStorage(), next);
    set({ notify: next });
  },
  reset() {
    saveNotify(browserStorage(), defaultNotify());
    set({ notify: defaultNotify() });
  },
}));
