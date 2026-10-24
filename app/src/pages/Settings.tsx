import { useState, type ReactNode } from 'react';
import { useT, type Key } from '../i18n';
import { AccessTab } from '../settings/AccessTab';
import { ClockTab } from '../settings/ClockTab';
import { ClusterTab } from '../settings/ClusterTab';
import { FiltersTab } from '../settings/FiltersTab';
import { FrequenciesTab } from '../settings/FrequenciesTab';
import { GeneralTab } from '../settings/GeneralTab';
import { AdvancedTab, AudioTab, RadioTab, ReportsTab } from '../settings/InfoTabs';
import { MacrosTab } from '../settings/MacrosTab';
import { NotificationsTab } from '../settings/NotificationsTab';
import { ScheduleTab } from '../settings/ScheduleTab';
import { SequenceTab } from '../settings/SequenceTab';
import { UpdatesTab } from '../settings/UpdatesTab';
import { WishlistTab } from '../settings/WishlistTab';
import { BandsTab, WindowsTab } from '../settings/WindowsBandsTabs';

interface TabDef {
  id: string;
  label: Key;
  tip: Key;
  render(): ReactNode;
}

/** Le schede, nell'ordine del motore, piu' quelle di Aurora (Finestre e Bande). */
const TABS: readonly TabDef[] = [
  { id: 'general', label: 'tab.general', tip: 'tip.tab.general', render: () => <GeneralTab /> },
  { id: 'radio', label: 'tab.radio', tip: 'tip.tab.radio', render: () => <RadioTab /> },
  { id: 'audio', label: 'tab.audio', tip: 'tip.tab.audio', render: () => <AudioTab /> },
  { id: 'sequence', label: 'tab.sequence', tip: 'tip.tab.sequence', render: () => <SequenceTab /> },
  { id: 'macros', label: 'tab.macros', tip: 'tip.tab.macros', render: () => <MacrosTab /> },
  { id: 'reports', label: 'tab.reports', tip: 'tip.tab.reports', render: () => <ReportsTab /> },
  { id: 'frequencies', label: 'tab.frequencies', tip: 'tip.tab.frequencies', render: () => <FrequenciesTab /> },
  { id: 'notifications', label: 'tab.notifications', tip: 'tip.tab.notifications', render: () => <NotificationsTab /> },
  { id: 'filters', label: 'tab.filters', tip: 'tip.tab.filters', render: () => <FiltersTab /> },
  { id: 'schedule', label: 'tab.schedule', tip: 'tip.tab.schedule', render: () => <ScheduleTab /> },
  { id: 'advanced', label: 'tab.advanced', tip: 'tip.tab.advanced', render: () => <AdvancedTab /> },
  { id: 'cluster', label: 'tab.cluster', tip: 'tip.tab.cluster', render: () => <ClusterTab /> },
  { id: 'wishlist', label: 'tab.wishlist', tip: 'tip.tab.wishlist', render: () => <WishlistTab /> },
  { id: 'updates', label: 'tab.updates', tip: 'tip.tab.updates', render: () => <UpdatesTab /> },
  { id: 'clock', label: 'tab.clock', tip: 'tip.tab.clock', render: () => <ClockTab /> },
  { id: 'access', label: 'tab.access', tip: 'tip.tab.access', render: () => <AccessTab /> },
  { id: 'windows', label: 'tab.windows', tip: 'tip.tab.windows', render: () => <WindowsTab /> },
  { id: 'bands', label: 'tab.bands', tip: 'tip.tab.bands', render: () => <BandsTab /> },
];

/** Impostazioni: una scheda per ogni funzione di configurazione, come nel motore. */
export function Settings({ initialTab }: { initialTab?: string }) {
  const t = useT();
  const [id, setId] = useState(TABS.some((x) => x.id === initialTab) ? (initialTab as string) : TABS[0]!.id);
  const tab = TABS.find((x) => x.id === id) ?? TABS[0]!;
  return (
    <div className="settings">
      <h1 className="page-title">{t('set.title')}</h1>
      <div className="tabs" role="tablist" aria-label={t('set.title')}>
        {TABS.map((x) => (
          <button
            key={x.id}
            type="button"
            role="tab"
            id={`tab-${x.id}`}
            aria-selected={tab.id === x.id}
            aria-controls="settings-panel"
            title={t(x.tip)}
            className={`glass${tab.id === x.id ? ' glass--on' : ''}`}
            onClick={() => setId(x.id)}
          >
            {t(x.label)}
          </button>
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
