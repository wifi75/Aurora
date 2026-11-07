import { UI_SCALES, useUiScale } from '../lib/uiScale';
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

      <section className="sw-box" aria-label={t('sw.station')}>
        <h3>{t('sw.station')}</h3>
        {/* prima riga: chi sei (nominativo, locatore, regione); seconda: lingua e tema piccoli a sinistra, i nominativi composti a destra */}
        <div className="sw-grid sw-station-who">
          {f('MyCall', 'text', 'gen2.call')}
          {f('MyGrid', 'text', 'gen2.grid')}
          {f('Region', 'choice', 'gen2.region', { options: REGION_OPTIONS })}
        </div>
        <div className="sw-grid sw-station-more">
          <div className="sw-field sw-field--compact">
            <span className="sw-label">{t('gen.language')}</span>
            <LanguageSelect />
          </div>
          <div className="sw-field sw-field--compact">
            <span className="sw-label">{t('gen.theme')}</span>
            <ThemeToggle />
          </div>
          <TextSize />
          {f('Type2MsgGen', 'choice', 'gen2.type2', { options: TYPE2_OPTIONS })}
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
    </div>
  );
}

/** Dimensione del testo di tutta l'interfaccia (anche con Ctrl +, Ctrl - e Ctrl 0). */
function TextSize() {
  const t = useT();
  const scale = useUiScale((s) => s.scale);
  return (
    <div className="sw-field sw-field--compact" title={t('tip.gen.textSize')}>
      <span className="sw-label">{t('gen.textSize')}</span>
      <div className="sw-scale" role="group" aria-label={t('gen.textSize')}>
        {UI_SCALES.map((v) => (
          <button key={v} type="button" className={`glass chip${scale === v ? ' glass--on' : ''}`} aria-pressed={scale === v} onClick={() => useUiScale.getState().set(v)}>
            {Math.round(v * 100)}%
          </button>
        ))}
      </div>
    </div>
  );
}
