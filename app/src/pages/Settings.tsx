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
import { LogServicesTab } from '../settings/LogServicesTab';
import { LINK_GROUPS, LOG_SERVICE_GROUPS, QSO_LOG_GROUPS, SPOT_GROUPS, type SettingGroup } from '../lib/engineSettingsLayout';
import { RadioTab } from '../settings/RadioTab';
import { MacrosTab } from '../settings/MacrosTab';
import { NotificationsTab } from '../settings/NotificationsTab';
import { ScheduleTab } from '../settings/ScheduleTab';
import { SequenceTab } from '../settings/SequenceTab';
import { OnlineTab } from '../settings/OnlineTab';
import { UpdatesTab } from '../settings/UpdatesTab';
import { AboutTab } from '../settings/AboutTab';
import { WishlistTab } from '../settings/WishlistTab';
import { SupportTab } from '../settings/SupportBox';
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
      ['notifications', <NotificationsTab />],
      ['wishlist', <WishlistTab />],
      ['dxcc', <DxccTab />],
      ['cluster', <ClusterTab />],
      ['reports', <ReportsTab />],
      ['online', <OnlineTab />],
      ['logservices', <LogServicesTab />],
      ['clock', <ClockTab />],
      ['updates', <UpdatesTab />],
      ['access', <AccessTab />],
      ['windows', <WindowsTab />],
      ['support', <SupportTab />],
      ['about', <AboutTab />],
    ] as const
  ).map(([id, node]) => [id, { id, label: `tab.${id}` as Key, tip: `tip.tab.${id}` as Key, render: () => node }]),
);

interface TabRow {
  id: string;
  title: Key;
  tabs: readonly string[];
}

/** La barra delle schede: sei gruppi per argomento, dalla stazione al sistema. */
const ROWS: readonly TabRow[] = [
  { id: 'station', title: 'tabg.station', tabs: ['general', 'radio', 'audio', 'clock'] },
  { id: 'rx', title: 'tabg.rx', tabs: ['waterfall', 'filters', 'bands', 'frequencies'] },
  { id: 'tx', title: 'tabg.tx', tabs: ['sequence', 'macros', 'schedule'] },
  { id: 'alerts', title: 'tabg.alerts', tabs: ['notifications', 'wishlist', 'dxcc'] },
  { id: 'net', title: 'tabg.net', tabs: ['cluster', 'online', 'logservices', 'reports'] },
  { id: 'system', title: 'tabg.system', tabs: ['access', 'windows', 'updates', 'support', 'about'] },
];

/** Le voci di ogni scheda che la ricerca trova oltre al nome e alla descrizione (anche quelle spostate da una scheda all'altra). */
const keysOf = (groups: readonly SettingGroup[]): Key[] => groups.flatMap((g) => [g.title, ...g.rows.map((r) => `ens.k.${r.key}` as Key)]);
const SEARCH: Partial<Record<string, readonly Key[]>> = {
  general: ['gen2.call', 'gen2.grid', 'gen2.monitorOff', 'gen2.logFilter', 'gen2.iaru', 'gen2.region', 'gen2.screen', 'gen.language', 'gen.theme'],
  sequence: [...keysOf(QSO_LOG_GROUPS), 'gen2.operation', 'gen2.watchdog', 'gen2.qsy', 'gen2.swr', 'gen2.wsjtx3', 'gen2.fullDuplex', 'gen2.waitFeatures', 'gen2.fox', 'gen2.otpVerify', 'gen2.bandHop', 'gen2.bandHopOn'],
  filters: ['fil.decode', 'gen2.decStart'],
  online: keysOf(SPOT_GROUPS),
  logservices: keysOf(LOG_SERVICE_GROUPS),
  reports: [...keysOf(LINK_GROUPS), 'int.title'],
};

const TABS: readonly TabDef[] = ROWS.flatMap((r) => r.tabs.map((id) => TAB[id]!));

/** Le vecchie scelte salvate: «Radio CAT» ora e' dentro «Radio», «Avanzate» non c'e' piu' (l'avvio della decodifica e' in «Filtri»). */
const RENAMED: Record<string, string> = { cat: 'radio', advanced: 'filters' };

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
  const matches = (x: TabDef) => needle === '' || [x.label, x.tip, ...(SEARCH[x.id] ?? [])].some((k) => t(k).toLowerCase().includes(needle));
  return (
    <div className="settings settings-layout">
      <aside className="settings-nav">
        <h1 className="settings-nav-title">{t('set.title')}</h1>
        <input type="search" className="settings-search" title={t('tip.ui.search')} placeholder={t('set.search')} aria-label={t('set.search')} value={query} onChange={(e) => setQuery(e.target.value)} />
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
        {tab.render()}
      </section>
    </div>
  );
}
