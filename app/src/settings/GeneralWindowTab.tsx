import { useT, type Key } from '../i18n';
import type { SettingRow } from '../lib/engineSettingsLayout';
import { PrefCheck, REGION_OPTIONS, ScreenExtras, TYPE2_OPTIONS } from './JtdxTabs';
import { SettingField, useDraft, useSettings } from './NativeSettings';
import { UpdatesTab } from './UpdatesTab';
import { Check } from './common';
import { usePrefs } from '../lib/prefs';

type Extra = { min?: number; max?: number; step?: number; options?: SettingRow['options'] };

/** Le parti della vecchia scheda Generale, ognuna nella sua pagina dell'albero delle Impostazioni. */
export type GeneralPart = 'all' | 'station' | 'qso' | 'screen' | 'updates';

/**
 * La vecchia scheda Generale, divisa in parti: dati della stazione (Stazione › Dati personali), registrazione dei QSO e comportamento
 * (Modi › Comune a tutti i modi), schermo (Modi › FT8 · FT4 › Messaggi e filtri) e aggiornamenti (Accesso e assistenza). Lingua, tema
 * e dimensione del testo stanno solo in Aspetto e avvisi › Aspetto.
 */
export function GeneralWindowTab({ part = 'all' }: { part?: GeneralPart } = {}) {
  const t = useT();
  const settings = useSettings();
  const draft = useDraft();
  const prefs = usePrefs((s) => s.prefs);
  const set = usePrefs.getState().set;
  // aggiornamenti e schermo non sono voci del motore: si vedono anche quando le impostazioni del motore non si leggono
  const updates = (
    <section className="sw-box" aria-label={t('tab.updates')}>
      <h3>{t('tab.updates')}</h3>
      <UpdatesTab />
      <p className="sw-note">{t('sw.updatesSelf')}</p>
    </section>
  );
  const screen = (
    <section className="sw-box" aria-label={t('gen.screen')}>
      <h3>{t('gen.screen')}</h3>
      <div className="sw-grid">
        <PrefCheck k="insertBlank" label={t('gen2.blank')} />
        <Check label={t('gen.miles')} hint={t('tip.gen.miles')} checked={prefs.distanceUnit === 'mi'} onChange={(v) => set({ distanceUnit: v ? 'mi' : 'km' })} />
        <PrefCheck k="showTx" label={t('gen2.showTx')} />
        <PrefCheck k="showCountry" label={t('gen.showCountry')} />
        <PrefCheck k="showDistance" label={t('gen.showDistance')} />
        <PrefCheck k="showAzimuth" label={t('gen2.azimuth')} />
        <ScreenExtras />
      </div>
    </section>
  );
  if (part === 'updates') return <div className="sw-page">{updates}</div>;
  if (part === 'screen') return <div className="sw-page">{screen}</div>;
  if (settings.isError) return <p className="notice">{t('ens.unavailable')}</p>;
  if (!settings.data) return <p className="muted">{t('set.loading')}</p>;
  const doc = settings.data;
  const f = (key: string, kind: 'bool' | 'text' | 'num' | 'choice', label: Key, extra: Extra = {}) => (
    <SettingField key={key} row={{ key, kind, ...extra }} label={t(label)} doc={doc} draft={draft} />
  );
  const station = (
    <section className="sw-box" aria-label={t('sw.station')}>
      <h3>{t('sw.station')}</h3>
      {/* chi sei: nominativo, locatore e regione; sotto i nominativi composti (lingua, tema e testo sono in Aspetto) */}
      <div className="sw-grid sw-station-who">
        {f('MyCall', 'text', 'gen2.call')}
        {f('MyGrid', 'text', 'gen2.grid')}
        {f('Region', 'choice', 'gen2.region', { options: REGION_OPTIONS })}
        {f('Type2MsgGen', 'choice', 'gen2.type2', { options: TYPE2_OPTIONS })}
      </div>
    </section>
  );
  const qso = (
    <>
      <section className="sw-box" aria-label={t('sw.behavior')}>
        <h3>{t('sw.behavior')}</h3>
        <div className="sw-grid">
          {f('MonitorOFF', 'bool', 'gen2.monitorOff')}
          {f('TxWatchdogTimer', 'num', 'gen2.watchdog', { min: 0, max: 99 })}
        </div>
      </section>
      <section className="sw-box" aria-label={t('sw.logging')}>
        <h3>{t('sw.logging')}</h3>
        <div className="sw-grid">
          {f('PromptToLog', 'bool', 'jx.PromptToLog')}
          {f('ClearCallGrid', 'bool', 'jx.ClearCallGrid')}
          {f('callsignLogFiltering', 'bool', 'gen2.useCall')}
          {f('gridLogFiltering', 'bool', 'gen2.useGrid')}
          {f('timeFromLogFiltering', 'text', 'gen2.fromDate')}
        </div>
      </section>
    </>
  );
  if (part === 'station') return <div className="sw-page">{station}</div>;
  if (part === 'qso') return <div className="sw-page">{qso}</div>;
  return (
    <div className="sw-page">
      {updates}
      {station}
      {screen}
      {qso}
    </div>
  );
}
