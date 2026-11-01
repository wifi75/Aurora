import { useT } from '../i18n';
import { JTDX_PALETTE_NAMES } from '../render/wfPalettes';
import { USER_PALETTE, WF_LIMITS, clampWf, paletteColors, useWfPrefs, type PaletteColor, type WfPrefs } from '../lib/wfPrefs';
import { Check, Help } from './common';
import { Cell, Cells, Field, Group, Notes, Page } from './layout';
import { WaterfallPreview } from './WaterfallPreview';

type NumberKey = keyof typeof WF_LIMITS;

function hex(c: PaletteColor): string {
  return `#${c.map((v) => v.toString(16).padStart(2, '0')).join('')}`;
}

function fromHex(value: string): PaletteColor {
  const n = Number.parseInt(value.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/** Una barra dei colori della tavolozza, per vederla prima di sceglierla. */
function Swatch({ colors }: { colors: ReadonlyArray<PaletteColor> }) {
  const stops = colors.map((c, i) => `rgb(${c[0]} ${c[1]} ${c[2]}) ${((i / Math.max(1, colors.length - 1)) * 100).toFixed(1)}%`).join(', ');
  return <span className="wfs-swatch" aria-hidden="true" style={{ background: `linear-gradient(90deg, ${stops})` }} />;
}

/** Scheda Cascata: le opzioni del Wide Graph di JTDX, applicate subito e ricordate al riavvio. */
export function WaterfallTab() {
  const t = useT();
  const prefs = useWfPrefs((s) => s.prefs);
  const set = useWfPrefs.getState().set;

  const number = (key: NumberKey, label: string, hint: string) => (
    <Field label={label} hint={hint}>
      <input
        type="number"
        className="set-number"
        min={WF_LIMITS[key][0]}
        max={WF_LIMITS[key][1]}
        step={key === 'startHz' ? 100 : key === 'spectrumPercent' ? 5 : 1}
        value={prefs[key]}
        onChange={(e) => {
          const n = Number(e.target.value);
          if (e.target.value !== '' && Number.isFinite(n)) set({ [key]: clampWf(key, n) } as Partial<WfPrefs>);
        }}
      />
    </Field>
  );

  const edit = (index: number, color: PaletteColor) => {
    const next = prefs.userPalette.map((c, i) => (i === index ? color : c));
    set({ userPalette: next, palette: USER_PALETTE });
  };

  return (
    <Page className="wfs-page">
      <Group title={t('wfs.preview.title')} span={3} className="wfs-preview-group">
        <WaterfallPreview />
      </Group>

      <Group title={t('wfs.res')}>
        <Cells min={14}>
          {number('binsPerPixel', t('wfs.bpp'), t('tip.wfs.bpp'))}
          {number('startHz', t('wfs.start'), t('tip.wfs.start'))}
          {number('navg', t('wfs.navg'), t('tip.wfs.navg'))}
        </Cells>
      </Group>

      <Group title={t('wfs.colors')} span={2}>
        <Cells min={15}>
          <Field label={t('wfs.palette')} hint={t('tip.wfs.palette')}>
            <select value={prefs.palette} onChange={(e) => set({ palette: e.target.value })}>
              {JTDX_PALETTE_NAMES.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
              <option value={USER_PALETTE}>{t('wfs.paletteUser')}</option>
            </select>
          </Field>
          <Cell>
            <Swatch colors={paletteColors(prefs)} />
          </Cell>
          {number('gain', t('wfs.gain'), t('tip.wfs.gain'))}
          {number('zero', t('wfs.zero'), t('tip.wfs.zero'))}
          <Check label={t('wfs.scale')} hint={t('tip.wfs.scale')} checked={prefs.scale} onChange={(v) => set({ scale: v })} />
          <Check label={t('wfs.flatten')} hint={t('tip.wfs.flatten')} checked={prefs.flatten} onChange={(v) => set({ flatten: v })} />
        </Cells>
      </Group>

      <Group title={t('wfs.spectrum')}>
        <Cells min={14}>
          {number('spectrumPercent', t('wfs.specPct'), t('tip.wfs.specPct'))}
          <Field label={t('wfs.specMode')} hint={t('tip.wfs.specMode')}>
            <select value={prefs.spectrumMode} onChange={(e) => set({ spectrumMode: e.target.value === 'cumulative' ? 'cumulative' : 'current' })}>
              <option value="current">{t('wfs.current')}</option>
              <option value="cumulative">{t('wfs.cumulative')}</option>
            </select>
          </Field>
          {number('gain2d', t('wfs.gain2d'), t('tip.wfs.gain2d'))}
          {number('zero2d', t('wfs.zero2d'), t('tip.wfs.zero2d'))}
        </Cells>
      </Group>

      <Group title={t('wfs.bars')}>
        <Cells min={15}>
          <Check label={t('wfs.barsOn')} hint={t('tip.wfs.barsOn')} checked={prefs.bars} onChange={(v) => set({ bars: v })} />
          <Check label={t('wfs.freqOn')} hint={t('tip.wfs.freqOn')} checked={prefs.freq} onChange={(v) => set({ freq: v })} />
          <Cell>
            <button type="button" className="glass" onClick={() => useWfPrefs.getState().reset()}>
              {t('set.reset')}
            </button>
          </Cell>
        </Cells>
      </Group>

      <Group title={t('wfs.palette.title')} span={2}>
        <Cell label={t('wfs.palette.title')} hint={t('tip.wfs.paletteEditor')} wide>
          <div className="wfs-editor">
            {prefs.userPalette.map((color, i) => (
              <span key={i} className="wfs-stop">
                <input type="color" aria-label={`${t('wfs.palette.title')} ${i + 1}`} value={hex(color)} onChange={(e) => edit(i, fromHex(e.target.value))} />
                <button
                  type="button"
                  className="glass"
                  disabled={prefs.userPalette.length <= 2}
                  aria-label={`${t('wfs.removeColor')} ${i + 1}`}
                  title={t('wfs.removeColor')}
                  onClick={() => set({ userPalette: prefs.userPalette.filter((_, k) => k !== i), palette: USER_PALETTE })}
                >
                  ×
                </button>
              </span>
            ))}
            <button
              type="button"
              className="glass"
              disabled={prefs.userPalette.length >= 255}
              onClick={() => set({ userPalette: [...prefs.userPalette, prefs.userPalette[prefs.userPalette.length - 1] ?? [255, 255, 255]], palette: USER_PALETTE })}
            >
              {t('wfs.addColor')}
            </button>
            <button type="button" className="glass" onClick={() => set({ userPalette: paletteColors(prefs).map((c) => [...c] as unknown as PaletteColor), palette: USER_PALETTE })}>
              {t('wfs.copyColors')}
            </button>
          </div>
        </Cell>
      </Group>

      <Notes>
        <Help>{t('wfs.note')}</Help>
      </Notes>
    </Page>
  );
}
