export type WaterfallSize = 'large' | 'small' | 'hidden';

const KEY = 'aurora-waterfall';

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

/** Dimensione ricordata della cascata; predefinita: grande. L'archivio puo' mancare o dare errore. */
export function readSize(storage: ReadableStorage): WaterfallSize {
  try {
    const value = storage?.getItem(KEY);
    return value === 'small' || value === 'hidden' ? value : 'large';
  } catch {
    return 'large';
  }
}

export function saveSize(storage: WritableStorage, size: WaterfallSize): void {
  try {
    storage?.setItem(KEY, size);
  } catch {
    // senza archivio la scelta non viene ricordata, ma l'interfaccia funziona lo stesso
  }
}
