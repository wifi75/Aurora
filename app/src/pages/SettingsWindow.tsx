import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { connectCat, postTune, putEngineSettings } from '../api/client';
import { useT, type Key } from '../i18n';
import { usePrefs } from '../lib/prefs';
import { rememberTab } from '../lib/lastPage';
import { browserStorage } from '../lib/theme';
import { AboutTab } from '../settings/AboutTab';
import { AccessTab } from '../settings/AccessTab';
import { ClusterTab } from '../settings/ClusterTab';
import { DxccTab } from '../settings/DxccTab';
import { FiltersTab } from '../settings/FiltersTab';
import { FrequenciesTab } from '../settings/FrequenciesTab';
import { GeneralWindowTab } from '../settings/GeneralWindowTab';
import { AudioTab, ReportsTab } from '../settings/InfoTabs';
import { LogServicesTab } from '../settings/LogServicesTab';
import { MacrosTab } from '../settings/MacrosTab';
import { NativeValues, saveErrorOf, useSettings, type SaveError } from '../settings/NativeSettings';
import { NotificationsTab } from '../settings/NotificationsTab';
import { OnlineTab } from '../settings/OnlineTab';
import { RadioTab } from '../settings/RadioTab';
import { ScheduleTab } from '../settings/ScheduleTab';
import { SequenceTab } from '../settings/SequenceTab';
import { SupportTab } from '../settings/SupportBox';
import { WaterfallTab } from '../settings/WaterfallTab';
import { WishlistTab } from '../settings/WishlistTab';
import { BandsTab, WindowsTab } from '../settings/WindowsBandsTabs';
import { SessionDraft, type SharedDraft } from '../settings/session';
import '../styles/settings-window.css';

interface TabDef {
  id: string;
  label: Key;
  icon: string;
  render(): ReactNode;
}

/** Contenuto di una scheda fatta di piu' schede vecchie, una sotto l'altra. */
const stack = (...parts: ReactNode[]): ReactNode => (
  <div className="sw-stack">
    {parts.map((p, i) => (
      <div key={i}>{p}</div>
    ))}
  </div>
);

/** Le 15 schede della finestra, nell'ordine del prototipo `docs/prototipo-impostazioni.html`. */
const TABS: readonly TabDef[] = [
  { id: 'general', label: 'sw.tab.general', icon: '⚙', render: () => <GeneralWindowTab /> },
  { id: 'radio', label: 'sw.tab.radio', icon: '▭', render: () => <RadioTab part="link" /> },
  { id: 'profiles', label: 'sw.tab.profiles', icon: '☰', render: () => <RadioTab part="profiles" /> },
  { id: 'audio', label: 'sw.tab.audio', icon: '∿', render: () => stack(<AudioTab />, <WaterfallTab />) },
  { id: 'macros', label: 'sw.tab.macros', icon: '≡', render: () => stack(<MacrosTab />, <SequenceTab />) },
  { id: 'reporting', label: 'sw.tab.reporting', icon: '⇄', render: () => stack(<ReportsTab />, <OnlineTab />, <ClusterTab />) },
  { id: 'freq', label: 'sw.tab.freq', icon: '∼', render: () => stack(<FrequenciesTab />, <BandsTab />) },
  { id: 'colors', label: 'sw.tab.colors', icon: '◐', render: () => <ColorsSoon /> },
  { id: 'logbooks', label: 'sw.tab.logbooks', icon: '▤', render: () => <LogServicesTab /> },
  { id: 'files', label: 'sw.tab.files', icon: '▣', render: () => <SupportTab /> },
  { id: 'autocq', label: 'sw.tab.autocq', icon: '◷', render: () => <ScheduleTab /> },
  { id: 'advanced', label: 'sw.tab.advanced', icon: '☼', render: () => <NativeValues group="advanced" /> },
  { id: 'alerts', label: 'sw.tab.alerts', icon: '♫', render: () => stack(<NotificationsTab />, <WishlistTab />, <DxccTab />) },
  { id: 'filters', label: 'sw.tab.filters', icon: '▽', render: () => <FiltersTab /> },
  { id: 'info', label: 'sw.tab.info', icon: 'ⓘ', render: () => stack(<AccessTab />, <WindowsTab />, <AboutTab />) },
];

function ColorsSoon() {
  const t = useT();
  return (
    <section className="sw-box">
      <p className="muted">{t('sw.soon')}</p>
    </section>
  );
}

/** Chiude la finestra: dal programma per Windows serve l'indirizzo speciale, `window.close()` da solo non basta. */
function closeWindow(): void {
  window.close();
  location.href = `${location.origin}/__chiudi-finestra`;
}

/**
 * Le Impostazioni in una finestra a parte, piccola: a sinistra le 15 schede, a destra la scheda scelta, in basso Prova CAT, Prova PTT,
 * Annulla, Applica e Salva. Le voci del motore scrivono in una bozza unica (`SessionDraft`) e partono solo con Applica o Salva.
 */
export function SettingsWindow({ initialTab }: { initialTab?: string }) {
  const t = useT();
  const client = useQueryClient();
  const settings = useSettings();
  const [id, setId] = useState(TABS.some((x) => x.id === initialTab) ? (initialTab as string) : 'general');
  const [changes, setChanges] = useState<Record<string, string>>({});
  const [error, setError] = useState<SaveError | null>(null);
  const [note, setNote] = useState('');
  const [busy, setBusy] = useState(false);
  // le preferenze dello schermo (distanza, bandiere...) agiscono subito: Annulla le rimette com'erano all'apertura
  const startPrefs = useRef(usePrefs.getState().prefs);

  useEffect(() => {
    document.title = `Aurora - ${t('set.title')}`;
  }, [t]);

  const draft = useMemo<SharedDraft>(
    () => ({
      changes,
      error,
      setError,
      set(key, value, current) {
        setError((e) => (e?.key === key ? null : e));
        setChanges((previous) => {
          const next = { ...previous };
          if (value === (current ?? '')) delete next[key];
          else next[key] = value;
          return next;
        });
      },
      clear: () => setChanges({}),
    }),
    [changes, error],
  );
  const count = Object.keys(changes).length;

  /** Manda la bozza al motore; vero se e' andata. */
  const apply = async (): Promise<boolean> => {
    if (count === 0) return true;
    setBusy(true);
    try {
      const next = await putEngineSettings(changes);
      const slow = Object.keys(changes).some((k) => settings.data?.items[k]?.apply === 'restart');
      client.setQueryData(['engine-settings'], next);
      setChanges({});
      setError(null);
      setNote(slow ? t('sw.appliedRestart') : t('sw.applied'));
      return true;
    } catch (e) {
      const err = saveErrorOf(e, t);
      if (err.key) setError(err);
      setNote(err.text);
      return false;
    } finally {
      setBusy(false);
    }
  };
  const save = async () => {
    if (await apply()) closeWindow();
  };
  const cancel = () => {
    usePrefs.getState().set(startPrefs.current);
    setChanges({});
    closeWindow();
  };

  const testCat = async () => {
    setNote('…');
    try {
      const snap = await connectCat();
      setNote(snap.status.state === 'connected' ? t('sw.catOk') : `${t('sw.catFail')} ${snap.status.message}`);
    } catch {
      setNote(t('sw.catFail'));
    }
  };
  const testPtt = async () => {
    if (!window.confirm(t('sw.pttConfirm'))) return;
    try {
      await postTune(true);
      await new Promise((r) => setTimeout(r, 1000));
      await postTune(false);
      setNote(t('sw.pttDone'));
    } catch {
      await postTune(false).catch(() => undefined);
      setNote(t('sw.pttFail'));
    }
  };

  const tab = TABS.find((x) => x.id === id) ?? TABS[0]!;
  return (
    <div className="sw-window" role="dialog" aria-label={t('set.title')}>
      <header className="sw-top">
        <strong>{t('set.title')}</strong>
        <small>{t(tab.label)}</small>
        <button type="button" className="sw-x" aria-label={t('sw.close')} title={t('sw.close')} onClick={cancel}>
          ✕
        </button>
      </header>
      <div className="sw-main">
        <nav className="sw-nav" role="tablist" aria-orientation="vertical">
          {TABS.map((x) => (
            <button
              key={x.id}
              type="button"
              role="tab"
              id={`swtab-${x.id}`}
              aria-selected={tab.id === x.id}
              aria-controls="sw-panel"
              className={tab.id === x.id ? 'sw-tab sw-tab--on' : 'sw-tab'}
              onClick={() => {
                setId(x.id);
                rememberTab(browserStorage(), x.id);
              }}
            >
              <i aria-hidden="true">{x.icon}</i>
              {t(x.label)}
            </button>
          ))}
        </nav>
        <section className="sw-panel settings settings-layout" id="sw-panel" role="tabpanel" aria-label={t(tab.label)}>
          <SessionDraft.Provider value={draft}>{tab.render()}</SessionDraft.Provider>
        </section>
      </div>
      <footer className="sw-foot">
        <button type="button" className="sw-btn" onClick={() => void testCat()}>
          {t('sw.testCat')}
        </button>
        <button type="button" className="sw-btn" onClick={() => void testPtt()}>
          {t('sw.testPtt')}
        </button>
        <span className="sw-state" role="status">
          {count > 0 ? t('sw.pending', { n: count }) : note}
        </span>
        <button type="button" className="sw-btn" disabled={busy} onClick={cancel}>
          {t('sw.cancel')}
        </button>
        <button type="button" className="sw-btn" disabled={busy || count === 0} onClick={() => void apply()}>
          {t('sw.apply')}
        </button>
        <button type="button" className="sw-btn sw-btn--save" disabled={busy} onClick={() => void save()}>
          {t('sw.save')}
        </button>
      </footer>
    </div>
  );
}
