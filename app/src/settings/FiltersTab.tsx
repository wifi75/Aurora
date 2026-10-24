import { useT, type Key } from '../i18n';
import { CONTINENTS, useFilters, type OnlyMode } from '../lib/filters';
import { Check, Help } from './common';

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
  const toggleContinent = (code: string) =>
    set({ hideContinents: filters.hideContinents.includes(code) ? filters.hideContinents.filter((c) => c !== code) : [...filters.hideContinents, code] });
  return (
    <>
      <Help>{t('fil.help')}</Help>
      <fieldset className="auto-tags" title={t('tip.fil.continent')}>
        <legend className="muted">{t('fil.continents')}</legend>
        <div className="set-checks set-checks--row">
          {CONTINENTS.map((code) => (
            <Check key={code} label={t(`cont.${code}` as Key)} checked={filters.hideContinents.includes(code)} onChange={() => toggleContinent(code)} />
          ))}
        </div>
      </fieldset>
      <label className="auto-field">
        {t('fil.countries')}
        <input type="text" value={filters.hideCountries.join(', ')} onChange={(e) => set({ hideCountries: list(e.target.value) })} />
      </label>
      <label className="auto-field">
        {t('fil.calls')}
        <input type="text" value={filters.hideCalls.join(', ')} onChange={(e) => set({ hideCalls: list(e.target.value) })} />
      </label>
      <label className="auto-field">
        {t('fil.only')}
        <select value={filters.only} onChange={(e) => set({ only: e.target.value as OnlyMode })}>
          {ONLY.map((o) => (
            <option key={o.value} value={o.value}>
              {t(o.label)}
            </option>
          ))}
        </select>
      </label>
      <div className="set-checks">
        <Check label={t('fil.nonStandard')} checked={filters.hideNonStandard} onChange={(v) => set({ hideNonStandard: v })} />
        <Check label={t('fil.ownContinent')} checked={filters.hideOwnContinent} onChange={(v) => set({ hideOwnContinent: v })} />
        <Check label={t('fil.worked')} checked={filters.hideWorked} onChange={(v) => set({ hideWorked: v })} />
      </div>
      <button type="button" className="glass" onClick={() => useFilters.getState().reset()}>
        {t('set.reset')}
      </button>
    </>
  );
}
