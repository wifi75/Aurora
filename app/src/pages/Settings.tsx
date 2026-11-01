import { useState, type ReactNode } from 'react';
import { useT, type Key } from '../i18n';
import { rememberTab } from '../lib/lastPage';
import { browserStorage } from '../lib/theme';
import { AccessTab } from '../settings/AccessTab';
import { ClockTab } from '../settings/ClockTab';
import { ClusterTab } from '../settings/ClusterTab';
import { DxccTab } from '../settings/DxccTab';
import { FiltersTab } from '../settings/FiltersTab';
import { FrequenciesTab } from '../settings/FrequenciesTab';
import { GeneralTab } from '../settings/GeneralTab';
import { AudioTab, ReportsTab } from '../settings/InfoTabs';
import { WaterfallTab } from '../settings/WaterfallTab';
import { AdvancedTab } from '../settings/AdvancedTab';
import { RadioTab } from '../settings/RadioTab';
import { MacrosTab } from '../settings/MacrosTab';
import { NotificationsTab } from '../settings/NotificationsTab';
import { ScheduleTab } from '../settings/ScheduleTab';
import { SequenceTab } from '../settings/SequenceTab';
import { OnlineTab } from '../settings/OnlineTab';
import { UpdatesTab } from '../settings/UpdatesTab';
import { AboutTab } from '../settings/AboutTab';
import { WishlistTab } from '../settings/WishlistTab';
import { BandsTab, WindowsTab } from '../settings/WindowsBandsTabs';

interface TabDef {
  id: string;
  label: Key;
  tip: Key;
  render(): ReactNode;
}

const TAB: Record<string, TabDef> = Object.fromEntries(
  (
    [
      ['general', <GeneralTab />],
      ['radio', <RadioTab />],
      ['audio', <AudioTab />],
      ['waterfall', <WaterfallTab />],
      ['frequencies', <FrequenciesTab />],
      ['bands', <BandsTab />],
      ['sequence', <SequenceTab />],
      ['macros', <MacrosTab />],
      ['schedule', <ScheduleTab />],
      ['filters', <FiltersTab />],
      ['advanced', <AdvancedTab />],
      ['notifications', <NotificationsTab />],
      ['wishlist', <WishlistTab />],
      ['dxcc', <DxccTab />],
      ['cluster', <ClusterTab />],
      ['reports', <ReportsTab />],
      ['online', <OnlineTab />],
      ['clock', <ClockTab />],
      ['updates', <UpdatesTab />],
      ['access', <AccessTab />],
      ['windows', <WindowsTab />],
      ['about', <AboutTab />],
    ] as const
  ).map(([id, node]) => [id, { id, label: `tab.${id}` as Key, tip: `tip.tab.${id}` as Key, render: () => node }]),
);

interface TabRow {
  id: string;
  title: Key;
  tabs: readonly string[];
}

/** La barra delle schede: due gruppi per tipo; in fondo Aggiornamenti e Informazioni. */
const ROWS: readonly TabRow[] = [
  { id: 'station', title: 'tabg.station', tabs: ['general', 'radio', 'audio', 'waterfall', 'frequencies', 'bands', 'sequence', 'macros', 'schedule', 'filters', 'advanced'] },
  { id: 'data', title: 'tabg.data', tabs: ['notifications', 'wishlist', 'dxcc', 'cluster', 'reports', 'online', 'clock', 'access', 'windows', 'updates', 'about'] },
];

const TABS: readonly TabDef[] = ROWS.flatMap((r) => r.tabs.map((id) => TAB[id]!));

/** Le vecchie scelte salvate: «Radio CAT» ora e' dentro «Radio». */
const RENAMED: Record<string, string> = { cat: 'radio' };

/**
 * Impostazioni: a sinistra l'elenco delle schede nei due gruppi, con una ricerca sul nome e sulla descrizione; a destra la scheda scelta.
 * Prima erano due righe di pulsanti grandi in cima, che occupavano mezza pagina.
 */
export function Settings({ initialTab }: { initialTab?: string }) {
  const t = useT();
  const wanted = initialTab ? (RENAMED[initialTab] ?? initialTab) : undefined;
  const [id, setId] = useState(TABS.some((x) => x.id === wanted) ? (wanted as string) : TABS[0]!.id);
  const [query, setQuery] = useState('');
  const tab = TABS.find((x) => x.id === id) ?? TABS[0]!;
  const needle = query.trim().toLowerCase();
  const matches = (x: TabDef) => needle === '' || `${t(x.label)} ${t(x.tip)}`.toLowerCase().includes(needle);
  return (
    <div className="settings settings-layout">
      <aside className="settings-nav">
        <h1 className="settings-nav-title">{t('set.title')}</h1>
        <input type="search" className="settings-search" placeholder={t('set.search')} aria-label={t('set.search')} value={query} onChange={(e) => setQuery(e.target.value)} />
        {ROWS.map((row) => {
          const shown = row.tabs.map((tid) => TAB[tid]!).filter(matches);
          if (shown.length === 0) return null;
          return (
            <div className="settings-nav-group" key={row.id}>
              <span className="settings-nav-label" id={`tabs-${row.id}`}>
                {t(row.title)}
              </span>
              <div role="tablist" aria-orientation="vertical" aria-labelledby={`tabs-${row.id}`}>
                {shown.map((x) => (
                  <button
                    key={x.id}
                    type="button"
                    role="tab"
                    id={`tab-${x.id}`}
                    aria-selected={tab.id === x.id}
                    aria-controls="settings-panel"
                    title={t(x.tip)}
                    className={`settings-nav-item${tab.id === x.id ? ' settings-nav-item--on' : ''}`}
                    onClick={() => {
                      setId(x.id);
                      rememberTab(browserStorage(), x.id);
                    }}
                  >
                    {t(x.label)}
                  </button>
                ))}
              </div>
            </div>
          );
        })}
        <a className="settings-nav-back" href="#/">
          {t('set.back')}
        </a>
      </aside>
      <section className="settings-content" role="tabpanel" id="settings-panel" aria-label={t(tab.label)}>
        <h2 className="settings-content-title">{t(tab.label)}</h2>
        <p className="muted settings-content-tip">{t(tab.tip)}</p>
        {tab.render()}
      </section>
    </div>
  );
}
