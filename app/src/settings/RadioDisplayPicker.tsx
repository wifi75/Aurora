import { RadioDisplay } from '../components/radio-display/RadioDisplay';
import { useT, type Key } from '../i18n';
import { DEMO_MODEL, RADIO_DISPLAY_STYLES, useRadioDisplay } from '../lib/radioDisplay';
import { Check, Help } from './common';

/** Scelta dello stile del display della radio, con un'anteprima (dati di prova) per ognuno, e l'opzione per nasconderlo. */
export function RadioDisplayPicker() {
  const t = useT();
  const pref = useRadioDisplay((s) => s.pref);
  const set = useRadioDisplay.getState().set;
  return (
    <>
      <h3 className="set-sub">{t('rd.title')}</h3>
      <Help>{t('rd.intro')}</Help>
      <div className="rd-picker" role="radiogroup" aria-label={t('rd.title')}>
        {RADIO_DISPLAY_STYLES.map((style) => (
          <label key={style} className={`rd-card${pref.style === style ? ' rd-card--on' : ''}${pref.hidden ? ' rd-card--off' : ''}`}>
            <input type="radio" name="radio-display-style" aria-label={t(`rd.style.${style}` as Key)} value={style} checked={pref.style === style} onChange={() => set({ style })} />
            <span className="rd-card-name">{t(`rd.style.${style}` as Key)}</span>
            <RadioDisplay model={DEMO_MODEL} style={style} size="full" />
          </label>
        ))}
      </div>
      <Help>{t('rd.preview')}</Help>
      <div className="set-checks">
        <Check label={t('rd.hide')} hint={t('tip.rd.hide')} checked={pref.hidden} onChange={(hidden) => set({ hidden })} />
      </div>
    </>
  );
}
