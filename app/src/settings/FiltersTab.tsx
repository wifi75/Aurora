import { useT, type Key } from '../i18n';
import { CQ_TAGS, useCqPrefs } from '../lib/cqPrefs';
import { CONTINENTS, useFilters, type OnlyMode } from '../lib/filters';
import { Check, Help } from './common';
import { Actions, Cells, Field, Group, Notes, Page } from './layout';

const ONLY: ReadonlyArray<{ value: OnlyMode; label: Key }> = [
  { value: 'all', label: 'fil.onlyAll' },
  { value: 'cq', label: 'fil.onlyCq' },
  { value: 'cq_rrr', label: 'fil.onlyCqRrr' },
  { value: 'cq_rr73', label: 'fil.onlyCqRr73' },
];

const list = (text: string) => text.split(',').map((x) => x.trim()).filter(Boolean);

/** Scheda Filtri: cosa nascondere o mostrare soltanto negli elenchi, come nel motore. */
export function FiltersTab() {
  const t = useT();
  const filters = useFilters((s) => s.filters);
  const set = useFilters.getState().set;
  const cq = useCqPrefs((s) => s.prefs);
  const setCq = useCqPrefs.getState().set;
  const csv = (text: string) => text.split(',').map((x) => x.trim().toUpperCase()).filter(Boolean);
  const toggleContinent = (code: string) =>
    set({ hideContinents: filters.hideContinents.includes(code) ? filters.hideContinents.filter((c) => c !== code) : [...filters.hideContinents, code] });
  return (
    <Page>
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
          <Check label={t('fil.nonStandard')} checked={filters.hideNonStandard} onChange={(v) => set({ hideNonStandard: v })} />
          <Check label={t('fil.ownContinent')} checked={filters.hideOwnContinent} onChange={(v) => set({ hideOwnContinent: v })} />
          <Check label={t('fil.worked')} checked={filters.hideWorked} onChange={(v) => set({ hideWorked: v })} />
        </Cells>
        <Actions>
          <button type="button" className="glass" onClick={() => useFilters.getState().reset()}>
            {t('set.reset')}
          </button>
        </Actions>
      </Group>
      <Group title={t('lay.lists')} span={3}>
        <Cells min={20}>
          <Field label={t('fil.countries')} stack>
            <input type="text" value={filters.hideCountries.join(', ')} onChange={(e) => set({ hideCountries: list(e.target.value) })} />
          </Field>
          <Field label={t('fil.calls')} stack>
            <input type="text" value={filters.hideCalls.join(', ')} onChange={(e) => set({ hideCalls: list(e.target.value) })} />
          </Field>
          <Field label={t('fil.only')} stack>
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
      <Group title={t('cqp.title')} span={3}>
        <Cells min={13}>
          {CQ_TAGS.map((tag) => (
            <Check key={tag} label={t(`kind.${tag}` as Key)} checked={cq.tags[tag]} onChange={(v) => setCq({ tags: { ...cq.tags, [tag]: v } })} />
          ))}
        </Cells>
        <Cells min={20}>
          <Field label={t('cqp.only')} stack>
            <input type="text" value={cq.only.join(', ')} onChange={(e) => setCq({ only: csv(e.target.value) })} />
          </Field>
          <Field label={t('cqp.hide')} stack>
            <input type="text" value={cq.hide.join(', ')} onChange={(e) => setCq({ hide: csv(e.target.value) })} />
          </Field>
          <Field label={t('cqp.limit')} stack>
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
      <Notes span={3}>
        <Help>{t('fil.help')}</Help>
      </Notes>
    </Page>
  );
}
