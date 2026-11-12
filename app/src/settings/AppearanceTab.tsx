import { useEffect, useState } from 'react';
import { LanguageSelect } from '../components/LanguageSelect';
import { useT } from '../i18n';
import {
  APPEARANCE_EVENT,
  contrastRatio,
  DATA_FONTS,
  DEFAULT_APPEARANCE,
  LOG_FONTS,
  LOG_SIZES,
  readAppearance,
  saveAppearance,
  TEXT_PALETTE,
  UI_FONTS,
  type Appearance,
} from '../lib/appearance';
import { browserStorage, chooseTheme, readThemeChoice, resolveTheme, THEME_EVENT, type ThemeChoice } from '../lib/theme';
import { useUiScale } from '../lib/uiScale';
import { Actions, Cell, Cells, Field, Group, Message, Page } from './layout';

/** Dimensioni del testo offerte qui: dal 90% al 140% a passi di 5. */
const SIZES = [0.9, 0.95, 1, 1.05, 1.1, 1.15, 1.2, 1.25, 1.3, 1.35, 1.4] as const;

/** Le superfici su cui sta il testo, per tema e contrasto (gli stessi valori di tokens.css e appearance.css): si controlla la peggiore. */
const SURFACES: Record<'dark' | 'light', Record<Appearance['contrast'], readonly string[]>> = {
  dark: { normal: ['#090c12', '#131b28', '#1b2536'], high: ['#121212', '#1c1c1c', '#262626'] },
  light: { normal: ['#d8dfe9', '#ffffff', '#eef2f8'], high: ['#f1f1f1', '#ffffff', '#e9ecef'] },
};
/** Sotto 4,5:1 (WCAG AA per il testo normale) si avvisa. */
const MIN_RATIO = 4.5;

/** Impostazioni > Aspetto: lingua, tema, alto contrasto, caratteri, dimensione e colore del testo, con anteprima immediata. */
export function AppearanceTab() {
  const t = useT();
  const [a, setA] = useState<Appearance>(() => readAppearance());
  const [choice, setChoice] = useState<ThemeChoice>(() => readThemeChoice(browserStorage()));
  const scale = useUiScale((s) => s.scale);

  // le scelte fatte in un'altra finestra (o dall'interruttore della testata) si vedono anche qui
  useEffect(() => {
    const syncA = () => setA(readAppearance());
    const syncT = () => setChoice(readThemeChoice(browserStorage()));
    const onStorage = (e: StorageEvent) => {
      if (e.key === 'aurora-appearance') syncA();
      if (e.key === 'aurora-theme') syncT();
    };
    window.addEventListener(APPEARANCE_EVENT, syncA);
    window.addEventListener(THEME_EVENT, syncT);
    window.addEventListener('storage', onStorage);
    return () => {
      window.removeEventListener(APPEARANCE_EVENT, syncA);
      window.removeEventListener(THEME_EVENT, syncT);
      window.removeEventListener('storage', onStorage);
    };
  }, []);

  const update = (patch: Partial<Appearance>) => {
    const next = { ...a, ...patch };
    setA(next);
    saveAppearance(next);
  };
  const pickTheme = (c: ThemeChoice) => {
    chooseTheme(c);
    setChoice(c);
  };

  const theme = resolveTheme(choice);
  const textKey: 'textLight' | 'textDark' = theme === 'light' ? 'textLight' : 'textDark';
  const textNow = a[textKey];
  const ratios = textNow ? SURFACES[theme][a.contrast].map((s) => contrastRatio(textNow, s)).filter((r): r is number => r !== null) : [];
  const worst = ratios.length ? Math.min(...ratios) : null;
  const setText = (c: string) => update(textKey === 'textLight' ? { textLight: c } : { textDark: c });

  const reset = () => {
    setA({ ...DEFAULT_APPEARANCE });
    saveAppearance({ ...DEFAULT_APPEARANCE });
    useUiScale.getState().set(1);
    pickTheme('dark');
  };

  const themes: { id: ThemeChoice; label: string }[] = [
    { id: 'dark', label: t('theme.dark') },
    { id: 'light', label: t('theme.light') },
    { id: 'auto', label: t('theme.auto') },
  ];
  const sizeIn = SIZES.some((s) => Math.abs(s - scale) < 0.001);

  return (
    <Page>
      <Group title={t('ap.theme')}>
        <Cells>
          {/* la lingua (prima nella scheda Generale) sta qui, con tema e testo: un posto solo per come si vede Aurora */}
          <Cell label={t('gen.language')} wide>
            <LanguageSelect />
          </Cell>
          <Cell label={t('ap.theme')} hint={t('tip.ap.theme')} wide>
            {themes.map((x) => (
              <button key={x.id} type="button" className={`glass chip${choice === x.id ? ' glass--on' : ''}`} aria-pressed={choice === x.id} onClick={() => pickTheme(x.id)}>
                {x.label}
              </button>
            ))}
          </Cell>
          <Field label={t('ap.contrast')} hint={t('tip.ap.contrast')} wide>
            <input type="checkbox" checked={a.contrast === 'high'} onChange={(e) => update({ contrast: e.target.checked ? 'high' : 'normal' })} />
          </Field>
        </Cells>
      </Group>

      <Group title={t('ap.text')}>
        <Cells>
          <Field label={t('ap.fontUi')} hint={t('tip.ap.fontUi')}>
            <select value={a.fontUi} onChange={(e) => update({ fontUi: e.target.value })}>
              {UI_FONTS.map((f) => (
                <option key={f.id} value={f.id} style={{ fontFamily: f.stack }}>
                  {f.label}
                </option>
              ))}
            </select>
          </Field>
          <Field label={t('ap.fontData')} hint={t('tip.ap.fontData')}>
            <select value={a.fontData} onChange={(e) => update({ fontData: e.target.value })}>
              {DATA_FONTS.map((f) => (
                <option key={f.id} value={f.id} style={{ fontFamily: f.stack }}>
                  {f.label}
                </option>
              ))}
            </select>
          </Field>
          <Field label={t('ap.fontLog')} hint={t('tip.ap.fontLog')}>
            <select value={a.fontLog} onChange={(e) => update({ fontLog: e.target.value })}>
              {LOG_FONTS.map((f) => (
                <option key={f.id} value={f.id} style={f.stack ? { fontFamily: f.stack } : undefined}>
                  {f.id === 'data' ? t('ap.fontLogData') : f.label}
                </option>
              ))}
            </select>
          </Field>
          <Field label={t('ap.logSize')} hint={t('tip.ap.logSize')}>
            <select value={String(a.logSize)} onChange={(e) => update({ logSize: Number(e.target.value) })}>
              {(LOG_SIZES as readonly number[]).includes(a.logSize) ? LOG_SIZES.map((s) => <option key={s} value={String(s)}>{s} px</option>) : [...LOG_SIZES, a.logSize].sort((x, y) => x - y).map((s) => <option key={s} value={String(s)}>{s} px</option>)}
            </select>
          </Field>
          <Field label={t('ap.size')} hint={t('tip.ap.size')}>
            <select value={String(sizeIn ? SIZES.find((s) => Math.abs(s - scale) < 0.001) : scale)} onChange={(e) => useUiScale.getState().set(Number(e.target.value))}>
              {/* una dimensione scelta con Ctrl + oltre il 140% resta nell'elenco */}
              {(sizeIn ? [...SIZES] : [...SIZES, scale]).map((s) => (
                <option key={s} value={String(s)}>
                  {Math.round(s * 100)}%
                </option>
              ))}
            </select>
          </Field>
          <Cell label={t('ap.color')} hint={t('tip.ap.color')} wide>
            <span className="ap-swatches">
              <button type="button" className={`glass chip${!textNow ? ' glass--on' : ''}`} aria-pressed={!textNow} onClick={() => setText('')}>
                {t('ap.colorTheme')}
              </button>
              {TEXT_PALETTE[theme].map((c) => (
                <button key={c} type="button" className="ap-swatch" style={{ background: c }} aria-pressed={textNow === c} aria-label={c} title={c} onClick={() => setText(c)} />
              ))}
              <input
                type="color"
                className="ap-color"
                value={textNow || (theme === 'light' ? '#0b1322' : '#e6edf8')}
                aria-label={t('ap.colorFree')}
                title={t('ap.colorFree')}
                onChange={(e) => setText(e.target.value)}
              />
            </span>
          </Cell>
        </Cells>
        {worst !== null && worst < MIN_RATIO ? <Message kind="alert">{t('ap.lowContrast', { ratio: worst.toFixed(1) })}</Message> : null}
        {worst !== null && worst >= MIN_RATIO ? <Message kind="muted">{t('ap.contrastOk', { ratio: worst.toFixed(1) })}</Message> : null}
      </Group>

      <Group title={t('ap.preview')} span={2}>
        <div className="ap-preview">
          <p>{t('ap.previewText')}</p>
          <p className="ap-muted">{t('ap.previewMuted')}</p>
          <p className="ap-data">14.074.000 · CQ DX K1ABC FN42 · −12 dB · 0.3 s · 1234 Hz</p>
          <p>
            <span className="ap-ok">● {t('ap.stOk')}</span> · <span className="ap-warn">● {t('ap.stWarn')}</span> · <span className="ap-danger">● {t('ap.stDanger')}</span> ·{' '}
            <span className="ap-accent">● {t('ap.stAccent')}</span>
          </p>
        </div>
        <Actions>
          <button type="button" className="glass" onClick={reset}>
            {t('ap.reset')}
          </button>
        </Actions>
      </Group>
    </Page>
  );
}
