import { LanguageSelect } from '../components/LanguageSelect';
import { ThemeToggle } from '../components/ThemeToggle';
import { useT } from '../i18n';
import { usePrefs } from '../lib/prefs';
import { useStation } from '../state/station';
import { JtdxValues } from './JtdxValues';
import { Check, Help, Row } from './common';

/** Scheda Generali: la stazione (letta da JTDX), come si mostrano i messaggi, lingua e tema. */
export function GeneralTab() {
  const t = useT();
  const prefs = usePrefs((s) => s.prefs);
  const set = usePrefs.getState().set;
  const status = useStation((s) => s.status);
  return (
    <>
      <h3 className="set-sub">{t('gen.station')}</h3>
      <Row name={t('gen.call')}>{status?.de_call || '—'}</Row>
      <Row name={t('gen.grid')}>{status?.de_grid || '—'}</Row>
      <Help>{t('gen.fromJtdx')}</Help>
      <JtdxValues group="general" />

      <h3 className="set-sub">{t('gen.screen')}</h3>
      <div className="set-checks">
        <Check label={t('gen.miles')} hint={t('tip.gen.miles')} checked={prefs.distanceUnit === 'mi'} onChange={(v) => set({ distanceUnit: v ? 'mi' : 'km' })} />
        <Check label={t('gen.showCountry')} hint={t('tip.gen.country')} checked={prefs.showCountry} onChange={(v) => set({ showCountry: v })} />
        <Check label={t('gen.showFlag')} hint={t('tip.gen.flag')} checked={prefs.showFlag} onChange={(v) => set({ showFlag: v })} />
        <Check label={t('gen.showDistance')} hint={t('tip.gen.distance')} checked={prefs.showDistance} onChange={(v) => set({ showDistance: v })} />
        <Check label={t('gen.showDt')} hint={t('tip.gen.dt')} checked={prefs.showDt} onChange={(v) => set({ showDt: v })} />
        <Check label={t('gen.escStop')} hint={t('tip.gen.escStop')} checked={prefs.escStop} onChange={(v) => set({ escStop: v })} />
        <Check label={t('gen.dblReply')} hint={t('tip.gen.dblReply')} checked={prefs.doubleClickReply} onChange={(v) => set({ doubleClickReply: v })} />
      </div>
      <label className="set-check" title={t('tip.gen.log4omCount')}>
        <span>{t('gen.log4omCount')}</span>
        <input
          type="number"
          className="set-number"
          min={1}
          max={200}
          value={prefs.log4omCount}
          onChange={(e) => {
            const n = Math.round(Number(e.target.value));
            if (Number.isFinite(n)) set({ log4omCount: Math.min(200, Math.max(1, n)) });
          }}
        />
      </label>
      <button type="button" className="glass" onClick={() => usePrefs.getState().reset()}>
        {t('set.reset')}
      </button>

      <h3 className="set-sub">{t('gen.language')}</h3>
      <LanguageSelect />
      <h3 className="set-sub">{t('gen.theme')}</h3>
      <ThemeToggle />
    </>
  );
}
