import { RadioDisplay } from '../components/radio-display/RadioDisplay';
import { useT, type Key } from '../i18n';
import { DEMO_MODEL, displaySize, isFaceStyle, isSquareStyle, RADIO_DISPLAY_LAYOUTS, RADIO_DISPLAY_STYLES, useRadioDisplay } from '../lib/radioDisplay';
import { Check, Help } from './common';

/** Scelta dello stile del display della radio, con un'anteprima (dati di prova) per ognuno, e l'opzione per nasconderlo. */
/** Le misure in testata (in punti), scritte accanto alla scelta: servono a dire «piu' largo», «piu' basso» sapendo da dove si parte. */
const LAYOUT_SIZE: Record<string, { w: number; h: number }> = { pair: { w: 614, h: 90 }, single: { w: 640, h: 90 } };
const FACE_SIZE = { w: 640, h: 90 };

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
          <label key={style} className={`rd-card${pref.style === style ? ' rd-card--on' : ''}${pref.hidden ? ' rd-card--off' : ''}${isFaceStyle(style) ? ' rd-card--wide' : ''}`}>
            <input type="radio" name="radio-display-style" aria-label={t(`rd.style.${style}` as Key)} value={style} checked={pref.style === style} onChange={() => set({ style })} />
            <span className="rd-card-name">{t(`rd.style.${style}` as Key)} <small className="rd-size">{t('rd.size', displaySize(style))}{isSquareStyle(style) ? ` · ${t('rd.square')}` : ''}</small></span>
            <RadioDisplay model={DEMO_MODEL} style={style} size="full" />
          </label>
        ))}
      </div>
      <Help>{t('rd.preview')}</Help>
      <Help>{t('tip.rd.faces')}</Help>
      <h3 className="set-sub">{t('rd.layout')}</h3>
      <Help>{t('tip.rd.layout')}</Help>
      {/* i pannelli FT-2000 e IC-7610 occupano da soli tutto lo spazio: la disposizione vale solo per gli altri stili */}
      {isFaceStyle(pref.style) ? <p className="muted">{t('rd.layoutFace')} {t('rd.size', FACE_SIZE)}</p> : null}
      <div className="rd-layouts" role="radiogroup" aria-label={t('rd.layout')} aria-disabled={isFaceStyle(pref.style) || undefined}>
        {RADIO_DISPLAY_LAYOUTS.map((layout) => (
          <label key={layout} className={`rd-layout-card${pref.layout === layout && !isFaceStyle(pref.style) ? ' rd-layout-card--on' : ''}${isFaceStyle(pref.style) ? ' rd-layout-card--off' : ''}`}>
            <input type="radio" name="radio-display-layout" value={layout} disabled={isFaceStyle(pref.style)} checked={pref.layout === layout && !isFaceStyle(pref.style)} onChange={() => set({ layout })} />
            <span>{t(`rd.layout.${layout}` as Key)}</span>
            <small className="rd-size">{t('rd.size', LAYOUT_SIZE[layout] ?? FACE_SIZE)}</small>
          </label>
        ))}
      </div>
      <div className="set-checks">
        <Check label={t('rd.hide')} hint={t('tip.rd.hide')} checked={pref.hidden} onChange={(hidden) => set({ hidden })} />
      </div>
    </>
  );
}
