import { LanguageSelect } from '../components/LanguageSelect';
import { ThemeToggle } from '../components/ThemeToggle';
import { useT, type Key } from '../i18n';
import type { SettingRow } from '../lib/engineSettingsLayout';
import { PrefCheck, REGION_OPTIONS, ScreenExtras, TYPE2_OPTIONS } from './JtdxTabs';
import { SettingField, useDraft, useSettings } from './NativeSettings';
import { UpdatesTab } from './UpdatesTab';
import { Check } from './common';
import { usePrefs } from '../lib/prefs';

type Extra = { min?: number; max?: number; step?: number; options?: SettingRow['options'] };

/**
 * La scheda Generale della finestra delle Impostazioni: aggiornamenti, dati della stazione, visualizzazione, comportamento e
 * registrazione dei QSO, in riquadri a tre colonne come nel prototipo `docs/prototipo-impostazioni.html`.
 */
export function GeneralWindowTab() {
  const t = useT();
  const settings = useSettings();
  const draft = useDraft();
  const prefs = usePrefs((s) => s.prefs);
  const set = usePrefs.getState().set;
  if (settings.isError) return <p className="notice">{t('ens.unavailable')}</p>;
  if (!settings.data) return <p className="muted">{t('set.loading')}</p>;
  const doc = settings.data;
  const f = (key: string, kind: 'bool' | 'text' | 'num' | 'choice', label: Key, extra: Extra = {}) => (
    <SettingField key={key} row={{ key, kind, ...extra }} label={t(label)} doc={doc} draft={draft} />
  );
  return (
    <div className="sw-page">
      <section className="sw-box" aria-label={t('tab.updates')}>
        <h3>{t('tab.updates')}</h3>
        <UpdatesTab />
        <p className="sw-note">{t('sw.updatesSelf')}</p>
      </section>

      <section className="sw-box" aria-label={t('gen2.station')}>
        <h3>{t('gen2.station')}</h3>
        <div className="sw-grid sw-grid--4">
          {f('MyCall', 'text', 'gen2.call')}
          {f('MyGrid', 'text', 'gen2.grid')}
          {f('Region', 'choice', 'gen2.region', { options: REGION_OPTIONS })}
          {f('Type2MsgGen', 'choice', 'gen2.type2', { options: TYPE2_OPTIONS })}
          <label className="sw-field">
            <span className="sw-label">{t('gen.language')}</span>
            <LanguageSelect />
          </label>
          <label className="sw-field">
            <span className="sw-label">{t('gen.theme')}</span>
            <ThemeToggle />
          </label>
        </div>
      </section>

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

      <section className="sw-box" aria-label={t('sw.behavior')}>
        <h3>{t('sw.behavior')}</h3>
        <div className="sw-grid">
          {f('MonitorOFF', 'bool', 'gen2.monitorOff')}
          {f('MonitorLastUsed', 'bool', 'jx.MonitorLastUsed')}
          {f('TxWatchdogTimer', 'num', 'gen2.watchdog', { min: 0, max: 99 })}
          {f('TuneTimer', 'num', 'jx.TuneTimer', { min: 0, max: 999 })}
        </div>
      </section>

      <section className="sw-box" aria-label={t('sw.logging')}>
        <h3>{t('sw.logging')}</h3>
        <div className="sw-grid">
          {f('PromptToLog', 'bool', 'jx.PromptToLog')}
          {f('AutoQSOLogging', 'bool', 'jx.AutoQSOLogging')}
          {f('ClearCallGrid', 'bool', 'jx.ClearCallGrid')}
          {f('callsignLogFiltering', 'bool', 'gen2.useCall')}
          {f('gridLogFiltering', 'bool', 'gen2.useGrid')}
          {f('timeFromLogFiltering', 'text', 'gen2.fromDate')}
        </div>
      </section>
    </div>
  );
}
