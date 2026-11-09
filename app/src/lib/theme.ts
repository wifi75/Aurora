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

/** La scelta dell'utente: un tema fisso o «automatico» (segue il tema chiaro/scuro del sistema). */
export type ThemeChoice = Theme | 'auto';

export function readThemeChoice(storage: ReadableStorage): ThemeChoice {
  try {
    const v = storage?.getItem(KEY);
    return v === 'light' || v === 'auto' ? v : 'dark';
  } catch {
    return 'dark';
  }
}

export function saveThemeChoice(storage: WritableStorage, choice: ThemeChoice): void {
  try {
    storage?.setItem(KEY, choice);
  } catch {
    // senza archivio la scelta non viene ricordata
  }
}

/** Il tema del sistema (Windows, telefono): scuro se non si sa. */
export function systemTheme(): Theme {
  try {
    return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
  } catch {
    return 'dark';
  }
}

export function resolveTheme(choice: ThemeChoice): Theme {
  return choice === 'auto' ? systemTheme() : choice;
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

/** Evento della stessa finestra quando il tema cambia fuori dall'interruttore (Impostazioni > Aspetto). */
export const THEME_EVENT = 'aurora-theme-change';

/** Sceglie, salva e applica il tema, e avvisa l'interruttore della testata. */
export function chooseTheme(choice: ThemeChoice): void {
  saveThemeChoice(browserStorage(), choice);
  applyTheme(document.documentElement, resolveTheme(choice));
  window.dispatchEvent(new CustomEvent(THEME_EVENT));
}
