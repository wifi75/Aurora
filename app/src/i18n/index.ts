import { useCallback } from 'react';
import { fill, pick, type Lang } from './core';
import { useLang } from './lang';
import { appStrings } from './strings/app';
import { audioStrings } from './strings/audio';
import { accessStrings } from './strings/access';
import { controlStrings } from './strings/controls';
import { clusterStrings } from './strings/cluster';
import { mapStrings } from './strings/map';
import { updateStrings } from './strings/update';
import { wishStrings } from './strings/wish';
import { catStrings } from './strings/cat';
import { clockStrings } from './strings/clock';
import { engineConfigStrings } from './strings/engineconfig';
import { diagnosisStrings } from './strings/diagnosis';
import { historyStrings } from './strings/history';
import { onlineStrings } from './strings/online';
import { radioDisplayStrings } from './strings/radiodisplay';
import { automationStrings } from './strings/automation';
import { outputStrings } from './strings/output';
import { settingsStrings } from './strings/settings';
import { stationStrings } from './strings/station';
import { statsStrings } from './strings/stats';
import { wikiStrings } from './strings/wiki';
import { wikiNewStrings } from './strings/wikinew';

export { DEFAULT_LANG, LANGS, LANG_NAMES, type Lang } from './core';
export { useLang } from './lang';

/** Tutti i testi dell'interfaccia, raccolti dalle tabelle per area. */
export const STRINGS = { ...appStrings, ...stationStrings, ...settingsStrings, ...automationStrings, ...accessStrings, ...outputStrings, ...statsStrings, ...wikiStrings, ...controlStrings, ...engineConfigStrings, ...clockStrings, ...catStrings, ...clusterStrings, ...mapStrings, ...wishStrings, ...updateStrings, ...audioStrings, ...historyStrings, ...onlineStrings, ...diagnosisStrings, ...radioDisplayStrings, ...wikiNewStrings } as const;

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
