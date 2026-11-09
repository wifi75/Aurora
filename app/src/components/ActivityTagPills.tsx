import { useT, type Key } from '../i18n';
import { CQ_TAG_LIST, useFilters } from '../lib/filters';

/**
 * Fila di pillole del pulsantino «Filtro» di Attività di banda (come GridTracker): si sceglie quali novità vedere,
 * logica «o»; «Tutte» toglie il filtro. La scelta e' quella dei filtri (si ricorda e va anche alle Automazioni).
 */
export function ActivityTagPills() {
  const t = useT();
  const tags = useFilters((s) => s.filters.tags);
  const set = useFilters.getState().set;
  const toggle = (tag: string) => set({ tags: tags.includes(tag) ? tags.filter((x) => x !== tag) : [...tags, tag] });
  return (
    <div className="bfl-pills" role="group" aria-label={t('bfl.title')}>
      <button type="button" className={tags.length === 0 ? 'bfl-pill bfl-pill--on' : 'bfl-pill'} aria-pressed={tags.length === 0} onClick={() => set({ tags: [] })}>
        {t('bfl.all')}
      </button>
      {CQ_TAG_LIST.map((tag) => (
        <button key={tag} type="button" className={tags.includes(tag) ? 'bfl-pill bfl-pill--on' : 'bfl-pill'} aria-pressed={tags.includes(tag)} onClick={() => toggle(tag)}>
          {t(`kind.${tag}` as Key)}
        </button>
      ))}
    </div>
  );
}
