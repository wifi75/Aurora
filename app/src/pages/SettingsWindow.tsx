import { useQueryClient } from '@tanstack/react-query';
import { Fragment, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { connectCat, postTune, putEngineSettings } from '../api/client';
import { useT, type Key } from '../i18n';
import { usePrefs } from '../lib/prefs';
import { rememberTab } from '../lib/lastPage';
import { browserStorage } from '../lib/theme';
import { AboutTab } from '../settings/AboutTab';
import { AccessTab } from '../settings/AccessTab';
import { ClockTab } from '../settings/ClockTab';
import { ClusterTab } from '../settings/ClusterTab';
import { DxccTab } from '../settings/DxccTab';
import { FiltersTab } from '../settings/FiltersTab';
import { FrequenciesTab } from '../settings/FrequenciesTab';
import { GeneralWindowTab } from '../settings/GeneralWindowTab';
import { AudioTab, ReportsTab } from '../settings/InfoTabs';
import { LogServicesTab } from '../settings/LogServicesTab';
import { MacrosTab } from '../settings/MacrosTab';
import { LogTransfer } from '../components/LogTransfer';
import { saveErrorOf, useSettings, type SaveError } from '../settings/NativeSettings';
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
  /** `extra`: funzioni che Aurora ha in piu' rispetto alle schede di riferimento, ognuna nella sua scheda e mai dentro un'altra. */
  extra?: boolean;
  /** L'ultima voce della barra, staccata dalle altre. */
  last?: boolean;
  render(): ReactNode;
}

/** Le schede di riferimento (ognuna con solo il suo contenuto) e poi quelle che sono solo di Aurora, nell'ordine di `docs/prototipo-impostazioni.html`. */
const TABS: readonly TabDef[] = [
  { id: 'general', label: 'sw.tab.general', icon: '⚙', render: () => <GeneralWindowTab /> },
  { id: 'radio', label: 'sw.tab.radio', icon: '▭', render: () => <RadioTab part="link" /> },
  { id: 'profiles', label: 'sw.tab.profiles', icon: '☰', render: () => <RadioTab part="profiles" /> },
  { id: 'audio', label: 'sw.tab.audio', icon: '∿', render: () => <AudioTab /> },
  { id: 'macros', label: 'sw.tab.macros', icon: '≡', render: () => <MacrosTab /> },
  { id: 'reporting', label: 'sw.tab.reporting', icon: '⇄', render: () => <ReportsTab /> },
  { id: 'freq', label: 'sw.tab.freq', icon: '∼', render: () => <FrequenciesTab /> },
  { id: 'colors', label: 'sw.tab.colors', icon: '◐', render: () => <NotificationsTab part="colors" /> },
  { id: 'logbooks', label: 'sw.tab.logbooks', icon: '▤', render: () => <LogServicesTab /> },
  { id: 'files', label: 'sw.tab.files', icon: '▣', render: () => <LogTransfer /> },
  { id: 'autocq', label: 'sw.tab.autocq', icon: '◷', render: () => <ScheduleTab /> },
  { id: 'alerts', label: 'sw.tab.alerts', icon: '♫', render: () => <NotificationsTab part="alerts" /> },
  { id: 'filters', label: 'sw.tab.filters', icon: '▽', render: () => <FiltersTab /> },
  // solo di Aurora
  { id: 'waterfall', label: 'tab.waterfall', icon: '▥', extra: true, render: () => <WaterfallTab /> },
  { id: 'sequence', label: 'tab.sequence', icon: '↻', extra: true, render: () => <SequenceTab /> },
  { id: 'bands', label: 'tab.bands', icon: '▦', extra: true, render: () => <BandsTab /> },
  { id: 'online', label: 'tab.online', icon: '☁', extra: true, render: () => <OnlineTab /> },
  { id: 'cluster', label: 'tab.cluster', icon: '✦', extra: true, render: () => <ClusterTab /> },
  { id: 'wishlist', label: 'tab.wishlist', icon: '★', extra: true, render: () => <WishlistTab /> },
  { id: 'dxcc', label: 'tab.dxcc', icon: '◎', extra: true, render: () => <DxccTab /> },
  { id: 'clock', label: 'tab.clock', icon: '◔', extra: true, render: () => <ClockTab /> },
  { id: 'windows', label: 'tab.windows', icon: '▢', extra: true, render: () => <WindowsTab /> },
  { id: 'phone', label: 'sw.tab.phone', icon: '☎', extra: true, render: () => <AccessTab /> },
  { id: 'support', label: 'tab.support', icon: '✚', extra: true, render: () => <SupportTab /> },
  // sempre l'ultima della barra
  { id: 'info', label: 'sw.tab.info', icon: 'ⓘ', last: true, render: () => <AboutTab /> },
];

/** Chiude la finestra: dal programma per Windows serve l'indirizzo speciale, `window.close()` da solo non basta. */
function closeWindow(): void {
  window.close();
  location.href = `${location.origin}/__chiudi-finestra`;
}

/**
 * Le Impostazioni in una finestra a parte, piccola: a sinistra le 15 schede, a destra la scheda scelta, in basso Prova CAT, Prova PTT,
 * Annulla, Applica e Salva. Le voci del motore scrivono in una bozza unica (`SessionDraft`) e partono solo con Applica o Salva.
 */
export function SettingsWindow({ initialTab, embedded = false, onClose }: { initialTab?: string; /** Dentro l'applicazione (riquadro sopra la pagina) e non in una finestra a parte. */ embedded?: boolean; onClose?(): void }) {
  const t = useT();
  const client = useQueryClient();
  const settings = useSettings();
  const [id, setId] = useState(TABS.some((x) => x.id === initialTab) ? (initialTab as string) : 'general');
  const [changes, setChanges] = useState<Record<string, string>>({});
  const [error, setError] = useState<SaveError | null>(null);
  const [note, setNote] = useState('');
  const [busy, setBusy] = useState(false);
  /** L'ultimo pulsante in basso premuto: resta evidenziato. */
  const [last, setLast] = useState('');
  const on = (name: string) => (last === name ? ' sw-btn--on' : '');
  // le preferenze dello schermo (distanza, bandiere...) agiscono subito: Annulla le rimette com'erano all'apertura
  const startPrefs = useRef(usePrefs.getState().prefs);

  useEffect(() => {
    document.title = `Aurora - ${t('set.title')}`;
  }, [t]);
  // finestra a parte: la pagina e' fissa (scorre solo il contenuto della scheda); nel riquadro non serve
  useEffect(() => {
    const cls = embedded ? 'sw-lock' : 'sw-fixed';
    document.documentElement.classList.add(cls);
    return () => document.documentElement.classList.remove(cls);
  }, [embedded]);
  const leave = () => (onClose ? onClose() : closeWindow());
  // Esc chiude (come Annulla)
  const cancelRef = useRef<() => void>(() => undefined);
  useEffect(() => {
    if (!embedded) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') cancelRef.current();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [embedded]);

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
    if (await apply()) leave();
  };
  const cancel = () => {
    usePrefs.getState().set(startPrefs.current);
    setChanges({});
    leave();
  };
  cancelRef.current = cancel;

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
  const body = (
    <div className={`sw-window${embedded ? ' sw-window--in' : ''}`} role="dialog" aria-label={t('set.title')}>
      <header className="sw-top" data-tauri-drag-region>
        <strong>{t('set.title')}</strong>
        <small>{t(tab.label)}</small>
        <button type="button" className="sw-x" aria-label={t('sw.close')} title={t('sw.close')} onClick={cancel}>
          ✕
        </button>
      </header>
      <div className="sw-main">
        <nav className="sw-nav" role="tablist" aria-orientation="vertical">
          {TABS.map((x, i) => (
            <Fragment key={x.id}>
              {x.extra && !TABS[i - 1]?.extra ? <span className="sw-nav-sep">{t('sw.extra')}</span> : null}
              {x.last ? <span className="sw-nav-sep sw-nav-sep--line" aria-hidden="true" /> : null}
              <button
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
            </Fragment>
          ))}
        </nav>
        <section className="sw-panel settings settings-layout" id="sw-panel" role="tabpanel" aria-label={t(tab.label)}>
          <SessionDraft.Provider value={draft}>{tab.render()}</SessionDraft.Provider>
        </section>
      </div>
      <footer className="sw-foot">
        <button type="button" className={`sw-btn${on('cat')}`} onClick={() => { setLast('cat'); void testCat(); }}>
          {t('sw.testCat')}
        </button>
        <button type="button" className={`sw-btn${on('ptt')}`} onClick={() => { setLast('ptt'); void testPtt(); }}>
          {t('sw.testPtt')}
        </button>
        <span className="sw-state" role="status">
          {count > 0 ? t('sw.pending', { n: count }) : note}
        </span>
        <button type="button" className="sw-btn" disabled={busy} onClick={cancel}>
          {t('sw.cancel')}
        </button>
        <button type="button" className={`sw-btn${on('apply')}`} disabled={busy || count === 0} onClick={() => { setLast('apply'); void apply(); }}>
          {t('sw.apply')}
        </button>
        <button type="button" className="sw-btn sw-btn--save" disabled={busy} onClick={() => void save()}>
          {t('sw.save')}
        </button>
      </footer>
    </div>
  );
  return embedded ? (
    <div className="settings-backdrop">
      <div className="settings-modal">{body}</div>
    </div>
  ) : (
    body
  );
}
