import { useCallback } from 'react';
import { fill, pick, type Lang } from './core';
import { useLang } from './lang';
import { NL_UI } from './nl/ui';
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
import { contestStrings } from './strings/contest';
import { qmapStrings } from './strings/qmap';
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
import { log4omDbStrings } from './strings/log4omdb';
import { onlineStrings } from './strings/online';
import { radioDisplayStrings } from './strings/radiodisplay';
import { automationStrings } from './strings/automation';
import { outputStrings } from './strings/output';
import { settingsStrings } from './strings/settings';
import { setWinStrings } from './strings/setwin';
import { sdrStrings } from './strings/sdr';
import { stationStrings } from './strings/station';
import { dxccStrings } from './strings/dxcc';
import { logPromptStrings } from './strings/logprompt';
import { checkAllStrings } from './strings/checkall';
import { coreTextStrings } from './strings/coretext';
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
import { firstRunStrings } from './strings/firstrun';
import { backupStrings } from './strings/backup';
import { setupCtaStrings } from './strings/setupcta';
import { appearanceStrings } from './strings/appearance';
import { rotatorStrings } from './strings/rotator';
import { digiStrings } from './strings/digi';
import { satStrings } from './strings/sat';
import { clusterOptsStrings } from './strings/clusteropts';
import { digiMenuStrings } from './strings/digimenu';
import { digiBandStrings } from './strings/digiband';
import { sstvStrings } from './strings/sstv';
import { lotwStrings } from './strings/lotw';
import { decodeOptStrings } from './strings/decodeopts';
import { systemStrings } from './strings/system';
import { isWindowsOs, useEngineOs } from './os';

export { DEFAULT_LANG, LANGS, LANG_NAMES, TABLE_LANGS, detectLang, langFromLocale, localeOf, type Lang } from './core';
export { useLang } from './lang';
export { useEngineOs } from './os';

/** Tutti i testi dell'interfaccia, raccolti dalle tabelle per area. */
export const STRINGS = { ...appStrings, ...commandStrings, ...stationStrings, ...settingsStrings, ...setWinStrings, ...sdrStrings, ...automationStrings, ...accessStrings, ...outputStrings, ...statsStrings, ...dxccStrings, ...dxccSetStrings, ...cqPrefsStrings, ...toastSetStrings, ...newsStrings, ...wikiStrings, ...controlStrings, ...engineConfigStrings, ...engineSettingsStrings, ...clockStrings, ...catStrings, ...clusterStrings, ...mapStrings, ...wishStrings, ...updateStrings, ...audioStrings, ...historyStrings, ...logExportStrings, ...log4omDbStrings, ...onlineStrings, ...diagnosisStrings, ...radioDisplayStrings, ...jtdxTabStrings, ...audioSetStrings, ...lookupStrings, ...layoutStrings, ...soundStrings, ...engineHintStrings, ...settingHintStrings, ...aboutStrings, ...notifyHintStrings, ...dxpedStrings, ...foxStrings, ...contestStrings, ...qmapStrings, ...radioExtraStrings, ...waterfallStrings, ...integrationStrings, ...phoneStrings, ...reportStrings, ...firstRunStrings, ...backupStrings, ...setupCtaStrings, ...appearanceStrings, ...rotatorStrings, ...sstvStrings, ...digiStrings, ...satStrings, ...clusterOptsStrings, ...digiMenuStrings, ...digiBandStrings, ...tooltipStrings, ...logPromptStrings, ...checkAllStrings, ...coreTextStrings, ...lotwStrings, ...decodeOptStrings, ...systemStrings } as const;

export type Key = keyof typeof STRINGS;

/** Il testo di una chiave nella lingua scelta; in olandese quello del dizionario `nl/ui.ts`, o l'inglese se manca. */
export function textOf(lang: Lang, key: Key): string | undefined {
  const entry = STRINGS[key];
  if (!entry) return undefined;
  return lang === 'nl' ? (NL_UI[key] ?? entry[1]) : pick(entry, lang);
}

/**
 * La chiave da usare sul sistema dove gira il motore: fuori da Windows (`otherSystem`) vale la variante `.sys` del testo, se c'e'
 * (i testi che nominano Windows: «volume di Windows» diventa «mixer audio del sistema», vedi `strings/system.ts`).
 */
export function keyForSystem(key: Key, otherSystem: boolean): Key {
  if (!otherSystem) return key;
  const variant = `${key}.sys`;
  return variant in STRINGS ? (variant as Key) : key;
}

export function translate(lang: Lang, key: Key, vars?: Record<string, string | number>, otherSystem = false): string {
  const text = textOf(lang, keyForSystem(key, otherSystem));
  return text === undefined ? key : fill(text, vars);
}

export type T = (key: Key, vars?: Record<string, string | number>) => string;

/** Funzione di traduzione che segue la lingua scelta (e il sistema del motore) e fa ridisegnare chi la usa quando cambiano. */
export function useT(): T {
  const lang = useLang((s) => s.lang);
  const otherSystem = useEngineOs((s) => !isWindowsOs(s.os));
  return useCallback((key, vars) => translate(lang, key, vars, otherSystem), [lang, otherSystem]);
}
