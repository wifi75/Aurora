import { CQ_TAGS, useCqPrefs } from '../lib/cqPrefs';
import { useT, type Key } from '../i18n';
import { CONTINENTS, CQ_TAG_LIST, useFilters, type OnlyMode } from '../lib/filters';
import { useBandFilter } from '../lib/bandFilter';
import { usePrefs } from '../lib/prefs';
import { Check, Help } from './common';
import { EngineValues } from './EngineValues';
import { NativeDecodeOptions } from './JtdxTabs';
import { useIsNative } from './NativeSettings';
import { Actions, Cells, Field, Group, Notes, Page } from './layout';

const ONLY: ReadonlyArray<{ value: OnlyMode; label: Key }> = [
  { value: 'all', label: 'fil.onlyAll' },
  { value: 'cq', label: 'fil.onlyCq' },
  { value: 'cq_rrr', label: 'fil.onlyCqRrr' },
  { value: 'cq_rr73', label: 'fil.onlyCqRr73' },
];

/** Pillola come nelle Automazioni: la casella resta per tastiera e lettori di schermo, si vede la pillola. */
function Pill({ label, checked, onChange }: { label: string; checked: boolean; onChange(): void }) {
  return (
    <label className="sw-check au-chip">
      <input type="checkbox" checked={checked} onChange={onChange} />
      {label}
    </label>
  );
}

const list = (text: string) => text.split(',').map((x) => x.trim()).filter(Boolean);

/** Scheda Filtri: cosa nascondere o mostrare soltanto negli elenchi, come nel motore. */
export function FiltersTab() {
  const t = useT();
  const filters = useFilters((s) => s.filters);
  const set = useFilters.getState().set;
  const csv = (text: string) => text.split(',').map((x) => x.trim().toUpperCase()).filter(Boolean);
  const native = useIsNative();
  const myBands = useBandFilter((s) => s.bands);
  const showTx = usePrefs((s) => s.prefs.showTx);
  const toggleIn = (items: string[], item: string) => (items.includes(item) ? items.filter((x) => x !== item) : [...items, item]);
  const num = (text: string, min: number, max: number): number | null => {
    if (text.trim() === '') return null;
    const n = Number(text);
    return Number.isFinite(n) ? Math.min(max, Math.max(min, Math.round(n))) : null;
  };
  const toggleContinent = (code: string) =>
    set({ hideContinents: filters.hideContinents.includes(code) ? filters.hideContinents.filter((c) => c !== code) : [...filters.hideContinents, code] });
  return (
    <Page>
      <Group title={t('bfl.title')} span={3}>
        <div className="au-chips" role="group" aria-label={t('bfl.tags')} title={t('tip.bfl.tags')}>
          <Pill label={t('bfl.all')} checked={filters.tags.length === 0} onChange={() => set({ tags: [] })} />
          {CQ_TAG_LIST.map((tag) => (
            <Pill key={tag} label={t(`kind.${tag}` as Key)} checked={filters.tags.includes(tag)} onChange={() => set({ tags: toggleIn(filters.tags, tag) })} />
          ))}
        </div>
        <Cells min={13}>
          <Field label={t('bfl.minSnr')} hint={t('tip.bfl.minSnr')} stack>
            <input type="number" min={-40} max={40} value={filters.minSnr ?? ''} placeholder="—" onChange={(e) => set({ minSnr: num(e.target.value, -40, 40) })} />
          </Field>
          <Field label={t('bfl.minKm')} hint={t('tip.bfl.km')} stack>
            <input type="number" min={0} max={25000} value={filters.minKm ?? ''} placeholder="—" onChange={(e) => set({ minKm: num(e.target.value, 0, 25000) })} />
          </Field>
          <Field label={t('bfl.maxKm')} hint={t('tip.bfl.km')} stack>
            <input type="number" min={0} max={25000} value={filters.maxKm ?? ''} placeholder="—" onChange={(e) => set({ maxKm: num(e.target.value, 0, 25000) })} />
          </Field>
          <Check label={t('bfl.hideToMe')} hint={t('tip.bfl.hideToMe')} checked={filters.hideToMe} onChange={(v) => set({ hideToMe: v })} />
          {/* le mie trasmissioni: e' la stessa scelta di «Mostra i miei Tx» delle preferenze, non una copia */}
          <Check label={t('bfl.hideOwnTx')} hint={t('tip.bfl.hideOwnTx')} checked={!showTx} onChange={(v) => usePrefs.getState().set({ showTx: !v })} />
        </Cells>
        <div title={t('tip.bfl.bands')}>
          <Field label={t('bfl.bands')} stack>
            <Cells min={7}>
              {myBands.map((band) => (
                <Check key={band} label={band} checked={filters.bands.includes(band)} onChange={() => set({ bands: toggleIn(filters.bands, band) })} />
              ))}
            </Cells>
          </Field>
        </div>
        <Help>{t('bfl.help')}</Help>
      </Group>
      <Group title={t('fil.continents')} span={2}>
        <div title={t('tip.fil.continent')}>
          <Cells min={10}>
            {CONTINENTS.map((code) => (
              <Check key={code} label={t(`cont.${code}` as Key)} checked={filters.hideContinents.includes(code)} onChange={() => toggleContinent(code)} />
            ))}
          </Cells>
        </div>
      </Group>
      <Group title={t('lay.options')}>
        <Cells min={13}>
          <Check label={t('fil.nonStandard')} hint={t('tip.fil.nonStandard')} checked={filters.hideNonStandard} onChange={(v) => set({ hideNonStandard: v })} />
          <Check label={t('fil.ownContinent')} hint={t('tip.fil.ownContinent')} checked={filters.hideOwnContinent} onChange={(v) => set({ hideOwnContinent: v })} />
          <Check label={t('fil.worked')} hint={t('tip.fil.worked')} checked={filters.hideWorked} onChange={(v) => set({ hideWorked: v })} />
          <Check label={t('fil.workedAll')} hint={t('tip.fil.workedAll')} checked={filters.hideWorkedAll} onChange={(v) => set({ hideWorkedAll: v })} />
          <Check label={t('fil.bypass')} hint={t('tip.fil.bypass')} checked={filters.bypass} onChange={(v) => set({ bypass: v })} />
        </Cells>
        <Actions>
          <button type="button" className="glass" onClick={() => useFilters.getState().reset()}>
            {t('set.reset')}
          </button>
        </Actions>
      </Group>
      <Group title={t('lay.lists')} span={3}>
        <Cells min={20}>
          <Field label={t('fil.countries')} hint={t('tip.fil.countries')} stack>
            <input type="text" value={filters.hideCountries.join(', ')} onChange={(e) => set({ hideCountries: list(e.target.value) })} />
          </Field>
          <Field label={t('fil.calls')} hint={t('tip.fil.calls')} stack>
            <input type="text" value={filters.hideCalls.join(', ')} onChange={(e) => set({ hideCalls: list(e.target.value) })} />
          </Field>
          <Field label={t('fil.showOnly')} hint={t('tip.fil.showOnly')} stack>
            <input type="text" value={filters.showOnlyCalls.join(', ')} onChange={(e) => set({ showOnlyCalls: csv(e.target.value) })} />
          </Field>
          <Field label={t('fil.always')} hint={t('tip.fil.always')} stack>
            <input type="text" value={filters.alwaysShowCalls.join(', ')} onChange={(e) => set({ alwaysShowCalls: csv(e.target.value) })} />
          </Field>
          <Field label={t('fil.only')} hint={t('tip.fil.only')} stack>
            <select value={filters.only} onChange={(e) => set({ only: e.target.value as OnlyMode })}>
              {ONLY.map((o) => (
                <option key={o.value} value={o.value}>
                  {t(o.label)}
                </option>
              ))}
            </select>
          </Field>
        </Cells>
      </Group>
      <Group title={t('fil.highlight')} span={3}>
        <Cells min={20}>
          <Field label={t('fil.hlCalls')} hint={t('tip.fil.hlCalls')} stack>
            <input type="text" value={filters.highlightCalls.join(', ')} onChange={(e) => set({ highlightCalls: csv(e.target.value) })} />
          </Field>
          <Field label={t('fil.hlCountries')} hint={t('tip.fil.hlCountries')} stack>
            <input type="text" value={filters.highlightCountries.join(', ')} onChange={(e) => set({ highlightCountries: list(e.target.value) })} />
          </Field>
        </Cells>
      </Group>
      {native ? <NativeDecodeOptions /> : null}
      {native === false ? (
        <Group title={t('fil.decode')} span={2}>
          <EngineValues group="advanced" />
        </Group>
      ) : null}
      <Notes span={3}>
        <Help>{t('fil.help')}</Help>
      </Notes>
    </Page>
  );
}

/** Scheda «CQ e novita'»: cosa elencare nella finestra «CQ e novita'» (prima era dentro Filtri e non si trovava). */
export function CqNewsTab() {
  const t = useT();
  const cq = useCqPrefs((s) => s.prefs);
  const setCq = useCqPrefs.getState().set;
  const csv = (text: string) => text.split(',').map((x) => x.trim().toUpperCase()).filter(Boolean);
  return (
    <Page>
      <Group title={t('cqp.title')} span={3}>
        <Cells min={13}>
          {CQ_TAGS.map((tag) => (
            <Check key={tag} label={t(`kind.${tag}` as Key)} checked={cq.tags[tag]} onChange={(v) => setCq({ tags: { ...cq.tags, [tag]: v } })} />
          ))}
        </Cells>
        <Cells min={20}>
          <Field label={t('cqp.only')} hint={t('tip.cqp.only')} stack>
            <input type="text" value={cq.only.join(', ')} onChange={(e) => setCq({ only: csv(e.target.value) })} />
          </Field>
          <Field label={t('cqp.hide')} hint={t('tip.cqp.hide')} stack>
            <input type="text" value={cq.hide.join(', ')} onChange={(e) => setCq({ hide: csv(e.target.value) })} />
          </Field>
          <Field label={t('cqp.limit')} hint={t('tip.cqp.limit')} stack>
            <input type="number" min={5} max={200} value={cq.limit} onChange={(e) => setCq({ limit: Math.min(200, Math.max(5, Number(e.target.value) || 40)) })} />
          </Field>
        </Cells>
        <Actions>
          <button type="button" className="glass" onClick={() => useCqPrefs.getState().reset()}>
            {t('set.reset')}
          </button>
        </Actions>
        <Help>{t('cqp.help')}</Help>
      </Group>
    </Page>
  );
}
