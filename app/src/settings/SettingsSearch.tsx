import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { useT, type Key } from '../i18n';
import { collectEntries, matchEntries, type SearchEntry } from '../lib/settingsSearch';

export interface SearchTab {
  id: string;
  label: Key;
  render(): ReactNode;
}

interface Props {
  tabs: readonly SearchTab[];
  /** Avvolge una scheda con quello che serve per disegnarla (la bozza delle impostazioni). */
  wrap(node: ReactNode): ReactNode;
  /** Apre la scheda; `text` e' l'opzione da mostrare. */
  onPick(tab: string, text: string): void;
}

/** Le voci gia' raccolte, per lingua: la lettura si fa una volta sola. */
const cache = new Map<string, SearchEntry[]>();

/**
 * Il campo «Cerca» delle Impostazioni. Alla prima ricerca disegna per un attimo tutte le schede fuori vista e ne legge i testi delle
 * opzioni (etichette, titoli, campi), poi le toglie: cosi' l'indice e' sempre quello vero, nella lingua in uso, senza elenchi da tenere.
 */
export function SettingsSearch({ tabs, wrap, onPick }: Props) {
  const t = useT();
  const lang = t('sw.search') as string;
  const [query, setQuery] = useState('');
  const [entries, setEntries] = useState<SearchEntry[] | null>(() => cache.get(lang) ?? null);
  const [reading, setReading] = useState(false);
  const holder = useRef<HTMLDivElement>(null);

  // la prima volta che si scrive si leggono le schede
  useEffect(() => {
    if (query.trim() === '' || entries !== null || reading) return;
    setReading(true);
  }, [query, entries, reading]);

  // le schede disegnate fuori vista hanno bisogno di un momento per caricare i loro dati
  useEffect(() => {
    if (!reading) return;
    const timer = window.setTimeout(() => {
      const root = holder.current;
      const found: SearchEntry[] = [];
      if (root) {
        for (const el of Array.from(root.querySelectorAll<HTMLElement>('[data-search-tab]'))) {
          found.push(...collectEntries(el.dataset.searchTab ?? '', el));
        }
      }
      cache.set(lang, found);
      setEntries(found);
      setReading(false);
    }, 1500);
    return () => window.clearTimeout(timer);
  }, [reading, lang]);

  const tabNames = useMemo(() => {
    const names = new Map<string, string>();
    for (const tab of tabs) names.set(tab.id, t(tab.label));
    return names;
  }, [tabs, t]);

  const results = useMemo(() => {
    if (query.trim() === '') return [];
    // anche i nomi delle schede si cercano
    const own: SearchEntry[] = tabs.map((tab) => ({ tab: tab.id, text: t(tab.label) }));
    return matchEntries([...own, ...(entries ?? [])], query);
  }, [query, entries, tabs, t]);

  return (
    <div className="sw-search">
      <input
        type="search"
        className="sw-search-input"
        value={query}
        placeholder={t('sw.search')}
        aria-label={t('sw.search')}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') setQuery('');
          if (e.key === 'Enter' && results[0]) {
            onPick(results[0].tab, results[0].text);
            setQuery('');
          }
        }}
      />
      {query.trim() !== '' ? (
        <ul className="sw-search-list" role="listbox" aria-label={t('sw.search')}>
          {reading ? <li className="sw-search-note">{t('sw.search.reading')}</li> : null}
          {results.map((r) => (
            <li key={`${r.tab}:${r.text}`}>
              <button
                type="button"
                role="option"
                aria-selected="false"
                onClick={() => {
                  onPick(r.tab, r.text);
                  setQuery('');
                }}
              >
                <b>{r.text}</b>
                <small>{tabNames.get(r.tab) ?? r.tab}</small>
              </button>
            </li>
          ))}
          {!reading && results.length === 0 ? <li className="sw-search-note">{t('sw.search.none')}</li> : null}
        </ul>
      ) : null}
      {reading ? (
        <div ref={holder} hidden aria-hidden="true">
          {tabs.map((tab) => (
            <div key={tab.id} data-search-tab={tab.id}>
              {wrap(tab.render())}
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}
