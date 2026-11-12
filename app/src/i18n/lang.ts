import { create } from 'zustand';
import { DEFAULT_LANG, detectLang, isLang, type Lang } from './core';

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

/** Segno del primo avvio gia' fatto (scritto dalla finestra di benvenuto). */
const FIRST_RUN_DONE_KEY = 'aurora-first-run-done';

/**
 * Primo avvio (nessuna lingua scelta ne' salvata): quella del sistema o del browser, se Aurora la conosce (`nl-BE` e `nl-NL` -> olandese).
 * Un'installazione che ha gia' fatto il primo avvio senza mai scegliere la lingua resta sull'italiano di sempre: non si cambia da sola.
 */
export function firstLaunchLang(storage: ReadableStorage, tags: readonly string[] | null | undefined): Lang | null {
  try {
    if (isLang(storage?.getItem(KEY))) return null;
    if (storage?.getItem(FIRST_RUN_DONE_KEY) === '1') return null;
  } catch {
    return null;
  }
  return detectLang(tags);
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
