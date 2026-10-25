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
import { AdvancedTab } from '../settings/AdvancedTab';
import { RadioTab } from '../settings/RadioTab';
import { MacrosTab } from '../settings/MacrosTab';
import { NotificationsTab } from '../settings/NotificationsTab';
import { ScheduleTab } from '../settings/ScheduleTab';
import { SequenceTab } from '../settings/SequenceTab';
import { OnlineTab } from '../settings/OnlineTab';
import { UpdatesTab } from '../settings/UpdatesTab';
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
    ] as const
  ).map(([id, node]) => [id, { id, label: `tab.${id}` as Key, tip: `tip.tab.${id}` as Key, render: () => node }]),
);

interface TabRow {
  id: string;
  title: Key;
  tabs: readonly string[];
}

/** La barra delle schede: due righe ordinate per tipo, dieci schede ciascuna. */
const ROWS: readonly TabRow[] = [
  { id: 'station', title: 'tabg.station', tabs: ['general', 'radio', 'audio', 'frequencies', 'bands', 'sequence', 'macros', 'schedule', 'filters', 'advanced'] },
  { id: 'data', title: 'tabg.data', tabs: ['notifications', 'wishlist', 'dxcc', 'cluster', 'reports', 'online', 'clock', 'updates', 'access', 'windows'] },
];

const TABS: readonly TabDef[] = ROWS.flatMap((r) => r.tabs.map((id) => TAB[id]!));

/** Le vecchie scelte salvate: «Radio CAT» ora e' dentro «Radio». */
const RENAMED: Record<string, string> = { cat: 'radio' };

/** Impostazioni: una scheda per ogni funzione di configurazione, in due righe per tipo; ogni scheda e' a riquadri e colonne. */
export function Settings({ initialTab }: { initialTab?: string }) {
  const t = useT();
  const wanted = initialTab ? (RENAMED[initialTab] ?? initialTab) : undefined;
  const [id, setId] = useState(TABS.some((x) => x.id === wanted) ? (wanted as string) : TABS[0]!.id);
  const tab = TABS.find((x) => x.id === id) ?? TABS[0]!;
  return (
    <div className="settings">
      <h1 className="page-title">{t('set.title')}</h1>
      <div className="tabs">
        {ROWS.map((row) => (
          <div className="tabs-row" key={row.id}>
            <span className="tabs-row-label" id={`tabs-${row.id}`}>
              {t(row.title)}
            </span>
            <div className="tabs-row-list" role="tablist" aria-labelledby={`tabs-${row.id}`}>
              {row.tabs.map((tid) => {
                const x = TAB[tid]!;
                return (
                  <button
                    key={x.id}
                    type="button"
                    role="tab"
                    id={`tab-${x.id}`}
                    aria-selected={tab.id === x.id}
                    aria-controls="settings-panel"
                    title={t(x.tip)}
                    className={`glass${tab.id === x.id ? ' glass--on' : ''}`}
                    onClick={() => {
                      setId(x.id);
                      rememberTab(browserStorage(), x.id);
                    }}
                  >
                    {t(x.label)}
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </div>
      <section className="panel settings-block" role="tabpanel" id="settings-panel" aria-label={t(tab.label)}>
        <h2 className="panel-title">{t(tab.label)}</h2>
        {tab.render()}
      </section>
      <a className="glass glass--link" href="#/">
        {t('set.back')}
      </a>
    </div>
  );
}
