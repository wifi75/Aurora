export type Theme = 'dark' | 'light';

const KEY = 'aurora-theme';

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

/** Il tema predefinito e' quello scuro ("Notte radio"); l'archivio puo' mancare o dare errore. */
export function readTheme(storage: ReadableStorage): Theme {
  try {
    return storage?.getItem(KEY) === 'light' ? 'light' : 'dark';
  } catch {
    return 'dark';
  }
}

export function saveTheme(storage: WritableStorage, theme: Theme): void {
  try {
    storage?.setItem(KEY, theme);
  } catch {
    // senza archivio il tema non viene ricordato, ma l'interfaccia funziona lo stesso
  }
}

export function applyTheme(root: HTMLElement, theme: Theme): void {
  root.dataset.theme = theme;
}

export function browserStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}
