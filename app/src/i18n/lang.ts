import { create } from 'zustand';
import { DEFAULT_LANG, isLang, type Lang } from './core';

const KEY = 'aurora-lang';

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

/** La lingua scelta; senza scelta, l'italiano. L'archivio puo' mancare o dare errore. */
export function readLang(storage: ReadableStorage): Lang {
  try {
    const value = storage?.getItem(KEY);
    return isLang(value) ? value : DEFAULT_LANG;
  } catch {
    return DEFAULT_LANG;
  }
}

export function saveLang(storage: WritableStorage, lang: Lang): void {
  try {
    storage?.setItem(KEY, lang);
  } catch {
    // senza archivio la lingua non viene ricordata, ma l'interfaccia funziona lo stesso
  }
}

function browserStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

interface LangState {
  lang: Lang;
  setLang(lang: Lang): void;
}

/** Lingua corrente, condivisa da tutta l'interfaccia: cambiarla aggiorna subito ogni testo. */
export const useLang = create<LangState>((set) => ({
  lang: readLang(browserStorage()),
  setLang(lang) {
    saveLang(browserStorage(), lang);
    document.documentElement.lang = lang;
    set({ lang });
  },
}));
