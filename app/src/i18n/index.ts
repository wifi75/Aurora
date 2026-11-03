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
import { commandStrings } from './strings/commands';
import { aboutStrings } from './strings/about';
import { notifyHintStrings } from './strings/notifyhints';
import { dxpedStrings } from './strings/dxped';
import { foxStrings } from './strings/fox';
import { tooltipStrings } from './strings/tooltips';
import { radioExtraStrings } from './strings/radioextras';
import { wishStrings } from './strings/wish';
import { catStrings } from './strings/cat';
import { clockStrings } from './strings/clock';
import { engineConfigStrings } from './strings/engineconfig';
import { engineSettingsStrings } from './strings/engineset';
import { diagnosisStrings } from './strings/diagnosis';
import { historyStrings } from './strings/history';
import { logExportStrings } from './strings/logexport';
import { onlineStrings } from './strings/online';
import { radioDisplayStrings } from './strings/radiodisplay';
import { automationStrings } from './strings/automation';
import { outputStrings } from './strings/output';
import { settingsStrings } from './strings/settings';
import { stationStrings } from './strings/station';
import { dxccStrings } from './strings/dxcc';
import { dxccSetStrings } from './strings/dxccset';
import { cqPrefsStrings } from './strings/cqprefs';
import { toastSetStrings } from './strings/toastset';
import { newsStrings } from './strings/news';
import { statsStrings } from './strings/stats';
import { wikiStrings } from './strings/wiki';
import { jtdxTabStrings } from './strings/jtdxtabs';
import { audioSetStrings } from './strings/audioset';
import { lookupStrings } from './strings/lookup';
import { layoutStrings } from './strings/layout';
import { soundStrings } from './strings/sounds';
import { engineHintStrings } from './strings/enginehints';
import { settingHintStrings } from './strings/settinghints';
import { waterfallStrings } from './strings/waterfall';
import { integrationStrings } from './strings/integrations';
import { phoneStrings } from './strings/phone';
import { reportStrings } from './strings/reports';

export { DEFAULT_LANG, LANGS, LANG_NAMES, type Lang } from './core';
export { useLang } from './lang';

/** Tutti i testi dell'interfaccia, raccolti dalle tabelle per area. */
export const STRINGS = { ...appStrings, ...commandStrings, ...stationStrings, ...settingsStrings, ...automationStrings, ...accessStrings, ...outputStrings, ...statsStrings, ...dxccStrings, ...dxccSetStrings, ...cqPrefsStrings, ...toastSetStrings, ...newsStrings, ...wikiStrings, ...controlStrings, ...engineConfigStrings, ...engineSettingsStrings, ...clockStrings, ...catStrings, ...clusterStrings, ...mapStrings, ...wishStrings, ...updateStrings, ...audioStrings, ...historyStrings, ...logExportStrings, ...onlineStrings, ...diagnosisStrings, ...radioDisplayStrings, ...jtdxTabStrings, ...audioSetStrings, ...lookupStrings, ...layoutStrings, ...soundStrings, ...engineHintStrings, ...settingHintStrings, ...aboutStrings, ...notifyHintStrings, ...dxpedStrings, ...foxStrings, ...radioExtraStrings, ...waterfallStrings, ...integrationStrings, ...phoneStrings, ...reportStrings, ...tooltipStrings } as const;

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
