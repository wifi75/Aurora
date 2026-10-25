import { LanguageSelect } from '../components/LanguageSelect';
import { ThemeToggle } from '../components/ThemeToggle';
import { useT } from '../i18n';
import { usePrefs } from '../lib/prefs';
import { useStation } from '../state/station';
import { EngineValues } from './EngineValues';
import { NativeGeneral } from './JtdxTabs';
import { useIsNative } from './NativeSettings';
import { RadioDisplayPicker } from './RadioDisplayPicker';
import { Check, Row } from './common';
import { Cell, Cells, Field, Group, Notes, Page } from './layout';
import { Help } from './common';

/** Scheda Generali: la stazione (letta dal motore), come si mostrano i messaggi, lingua e tema. */
export function GeneralTab() {
  const native = useIsNative();
  return native ? <NativeGeneral /> : <JtdxGeneralTab />;
}

function JtdxGeneralTab() {
  const t = useT();
  const prefs = usePrefs((s) => s.prefs);
  const set = usePrefs.getState().set;
  const status = useStation((s) => s.status);
  return (
    <Page>
      <Group title={t('gen.station')}>
        <Cells min={12}>
          <Row name={t('gen.call')}>{status?.de_call || '—'}</Row>
          <Row name={t('gen.grid')}>{status?.de_grid || '—'}</Row>
        </Cells>
      </Group>

      <Group title={t('gen.screen')} span={2}>
        <Cells min={15}>
          <Check label={t('gen.miles')} hint={t('tip.gen.miles')} checked={prefs.distanceUnit === 'mi'} onChange={(v) => set({ distanceUnit: v ? 'mi' : 'km' })} />
          <Check label={t('gen.showCountry')} hint={t('tip.gen.country')} checked={prefs.showCountry} onChange={(v) => set({ showCountry: v })} />
          <Check label={t('gen.showFlag')} hint={t('tip.gen.flag')} checked={prefs.showFlag} onChange={(v) => set({ showFlag: v })} />
          <Check label={t('gen.showDistance')} hint={t('tip.gen.distance')} checked={prefs.showDistance} onChange={(v) => set({ showDistance: v })} />
          <Check label={t('gen.showDt')} hint={t('tip.gen.dt')} checked={prefs.showDt} onChange={(v) => set({ showDt: v })} />
          <Check label={t('gen.escStop')} hint={t('tip.gen.escStop')} checked={prefs.escStop} onChange={(v) => set({ escStop: v })} />
          <Check label={t('gen.clearOnChange')} hint={t('tip.gen.clearOnChange')} checked={prefs.clearOnChange} onChange={(v) => set({ clearOnChange: v })} />
          <Check label={t('gen.dblReply')} hint={t('tip.gen.dblReply')} checked={prefs.doubleClickReply} onChange={(v) => set({ doubleClickReply: v })} />
          <Field label={t('gen.log4omCount')} hint={t('tip.gen.log4omCount')}>
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
          </Field>
          <Cell>
            <button type="button" className="glass" onClick={() => usePrefs.getState().reset()}>
              {t('set.reset')}
            </button>
          </Cell>
        </Cells>
      </Group>

      <Group title={t('gen.language')}>
        <LanguageSelect />
      </Group>
      <Group title={t('gen.theme')}>
        <ThemeToggle />
      </Group>

      <Group title={t('jx.title')} span={2}>
        <EngineValues group="general" />
      </Group>

      <Group title={t('rd.title')} span={3}>
        <RadioDisplayPicker />
      </Group>

      <Notes>
        <Help>{t('gen.fromEngine')}</Help>
      </Notes>
    </Page>
  );
}
