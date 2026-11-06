import { useT } from '../i18n';
import { JTDX_PALETTES } from '../render/wfPalettes';
import { WF_LIMITS, USER_PALETTE, clampWf, useWfPrefs, type WfPrefs } from '../lib/wfPrefs';
import { useSpectrumSpan, visibleRange } from '../lib/spectrumFeed';

type NumberKey = keyof typeof WF_LIMITS;

/**
 * La barra sopra la cascata: Bins/Pixel, Start, Palette, Gain, Zero, N Avg, Spec, Flatten e Time stamp, con la banda visibile.
 * Sono le stesse opzioni di Impostazioni > Cascata (stesso archivio `useWfPrefs`): cambiare qui o li' e' la stessa cosa.
 */
export function WaterfallToolbar() {
  const t = useT();
  const prefs = useWfPrefs((s) => s.prefs);
  const spanHz = useSpectrumSpan((s) => s.spanHz);
  const { from: fromHz, to: toHz } = visibleRange(prefs.startHz, spanHz);
  const set = useWfPrefs.getState().set;
  const number = (key: NumberKey, label: string, tip: string, width = 2.6) => (
    <label className="wft-item" title={tip}>
      <span>{label}</span>
      <input
        type="number"
        className="wft-num"
        style={{ width: `${width}rem` }}
        min={WF_LIMITS[key][0]}
        max={WF_LIMITS[key][1]}
        step={key === 'startHz' ? 100 : 1}
        value={prefs[key]}
        aria-label={label}
        onChange={(e) => {
          const n = Number(e.target.value);
          if (e.target.value !== '' && Number.isFinite(n)) set({ [key]: clampWf(key, n) } as Partial<WfPrefs>);
        }}
      />
    </label>
  );
  const slider = (key: 'gain' | 'zero', label: string, tip: string) => (
    <label className="wft-item" title={tip}>
      <span>{label}</span>
      <input
        type="range"
        className="wft-range"
        min={WF_LIMITS[key][0]}
        max={WF_LIMITS[key][1]}
        step={1}
        value={prefs[key]}
        aria-label={label}
        onChange={(e) => set({ [key]: clampWf(key, Number(e.target.value)) } as Partial<WfPrefs>)}
      />
    </label>
  );
  return (
    <div className="wf-toolbar" role="group" aria-label={t('wft.aria')}>
      {number('binsPerPixel', t('wft.bins'), t('tip.wfs.bpp'))}
      {number('startHz', t('wft.start'), t('tip.wfs.start'), 3.2)}
      <label className="wft-item" title={t('tip.wfs.palette')}>
        <span>{t('wft.palette')}</span>
        <select className="wft-select" value={prefs.palette} aria-label={t('wft.palette')} onChange={(e) => set({ palette: e.target.value })}>
          {Object.keys(JTDX_PALETTES).map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
          <option value={USER_PALETTE}>{t('wfs.paletteUser')}</option>
        </select>
      </label>
      {slider('gain', t('wft.gain'), t('tip.wfs.gain'))}
      {slider('zero', t('wft.zero'), t('tip.wfs.zero'))}
      {number('navg', t('wft.navg'), t('tip.wfs.navg'))}
      <label className="wft-item" title={t('tip.wfs.specMode')}>
        <span>{t('wft.spec')}</span>
        <select
          className="wft-select"
          value={prefs.spectrumMode}
          aria-label={t('wft.spec')}
          onChange={(e) => set({ spectrumMode: e.target.value === 'cumulative' ? 'cumulative' : 'current' })}
        >
          <option value="current">{t('wfs.current')}</option>
          <option value="cumulative">{t('wfs.cumulative')}</option>
        </select>
      </label>
      <label className="wft-item wft-check" title={t('tip.wfs.flatten')}>
        <input type="checkbox" checked={prefs.flatten} onChange={(e) => set({ flatten: e.target.checked })} />
        <span>{t('wft.flatten')}</span>
      </label>
      <label className="wft-item wft-check" title={t('tip.wft.stamp')}>
        <input type="checkbox" checked={prefs.timeStamp} onChange={(e) => set({ timeStamp: e.target.checked })} />
        <span>{t('wft.stamp')}</span>
      </label>
      <span className="wft-range-text" aria-live="off">
        {`${fromHz}–${toHz} Hz`}
      </span>
    </div>
  );
}
