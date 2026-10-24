import { useCallback } from 'react';
import { fill, pick, type Lang } from './core';
import { useLang } from './lang';
import { appStrings } from './strings/app';
import { stationStrings } from './strings/station';

export { DEFAULT_LANG, LANGS, LANG_NAMES, type Lang } from './core';
export { useLang } from './lang';

/** Tutti i testi dell'interfaccia, raccolti dalle tabelle per area. */
export const STRINGS = { ...appStrings, ...stationStrings } as const;

export type Key = keyof typeof STRINGS;

export function translate(lang: Lang, key: Key, vars?: Record<string, string | number>): string {
  const entry = STRINGS[key];
  return entry ? fill(pick(entry, lang), vars) : key;
}

export type T = (key: Key, vars?: Record<string, string | number>) => string;

/** Funzione di traduzione che segue la lingua scelta e fa ridisegnare chi la usa quando cambia. */
export function useT(): T {
  const lang = useLang((s) => s.lang);
  return useCallback((key, vars) => translate(lang, key, vars), [lang]);
}
