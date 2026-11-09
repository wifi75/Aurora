import { useT, type Key } from '../i18n';
import { CQ_TAG_LIST, useFilters } from '../lib/filters';

/**
 * Fila di pillole del pulsantino «Filtro» di Attività di banda (come GridTracker): si sceglie quali novità vedere,
 * logica «o»; «Tutte» toglie il filtro. La scelta e' quella dei filtri (si ricorda e va anche alle Automazioni).
 */
export function ActivityTagPills() {
  const t = useT();
  const tags = useFilters((s) => s.filters.tags);
  const f = useFilters((s) => s.filters);
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
      <span className="bfl-sep" aria-hidden="true" />
      <span className="bfl-group">{t('bfl.except')}</span>
      {/* le «eccezioni» come in GridTracker (richiesta di ON7KEC): restringono quello che resta */}
      <button type="button" className={f.only === 'cq' ? 'bfl-pill bfl-pill--on' : 'bfl-pill'} aria-pressed={f.only === 'cq'} onClick={() => set({ only: f.only === 'cq' ? 'all' : 'cq' })}>
        {t('bfl.cqOnly')}
      </button>
      <button type="button" className={f.only === 'cq_rr73' ? 'bfl-pill bfl-pill--on' : 'bfl-pill'} aria-pressed={f.only === 'cq_rr73'} onClick={() => set({ only: f.only === 'cq_rr73' ? 'all' : 'cq_rr73' })}>
        {t('bfl.rr73AsCq')}
      </button>
      <button type="button" className={f.hasGrid ? 'bfl-pill bfl-pill--on' : 'bfl-pill'} aria-pressed={f.hasGrid} onClick={() => set({ hasGrid: !f.hasGrid })}>
        {t('bfl.hasGrid')}
      </button>
      <button type="button" className={f.hideOwnContinent ? 'bfl-pill bfl-pill--on' : 'bfl-pill'} aria-pressed={f.hideOwnContinent} onClick={() => set({ hideOwnContinent: !f.hideOwnContinent })}>
        {t('bfl.notMyCont')}
      </button>
    </div>
  );
}
