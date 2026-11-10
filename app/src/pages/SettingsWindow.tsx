import { useStation } from '../state/station';
import { askConfirm } from '../components/ConfirmHost';
import { useQueryClient } from '@tanstack/react-query';
import { Fragment, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { connectCat, getCat, postTune, putAutomation, putEngineSettings } from '../api/client';
import type { AutomationRules } from '../api/automation';
import { useT, type Key } from '../i18n';
import { usePrefs } from '../lib/prefs';
import { rememberTab } from '../lib/lastPage';
import { SETTINGS_TAB_EVENT } from '../lib/settingsWindow';
import { browserStorage } from '../lib/theme';
import { AboutTab } from '../settings/AboutTab';
import { AccessTab } from '../settings/AccessTab';
import { AutomationTab, AUTOMATION_DRAFT } from '../settings/AutomationTab';
import { ClockTab } from '../settings/ClockTab';
import { ClusterTab } from '../settings/ClusterTab';
import { DxccTab } from '../settings/DxccTab';
import { CqNewsTab, FiltersTab } from '../settings/FiltersTab';
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
import { RotatorTab } from '../settings/RotatorTab';
import { SdrTab } from '../settings/SdrTab';
import { ScheduleTab } from '../settings/ScheduleTab';
import { SequenceTab } from '../settings/SequenceTab';
import { SupportTab } from '../settings/SupportBox';
import { AgentTab } from '../settings/AgentTab';
import { AppearanceTab } from '../settings/AppearanceTab';
import { WaterfallTab } from '../settings/WaterfallTab';
import { WishlistTab } from '../settings/WishlistTab';
import { BandsTab, WindowsTab } from '../settings/WindowsBandsTabs';
import { SessionDraft, type SharedDraft } from '../settings/session';
import '../styles/settings-window.css';

interface TabDef {
  id: string;
  label: Key;
  icon: string;
  /** Il gruppo della barra: il titolo compare davanti alla prima scheda di ogni gruppo. */
  section?: Key;
  /** L'ultima voce della barra, staccata dalle altre. */
  last?: boolean;
  render(): ReactNode;
}

/** Le schede di riferimento (ognuna con solo il suo contenuto) e poi quelle che sono solo di Aurora, nell'ordine di `docs/prototipo-impostazioni.html`. */
const TABS: readonly TabDef[] = [
  { id: 'general', section: 'sw.sec.station', label: 'sw.tab.general', icon: '⚙', render: () => <GeneralWindowTab /> },
  { id: 'radio', section: 'sw.sec.station', label: 'sw.tab.radio', icon: '▭', render: () => <RadioTab part="link" /> },
  { id: 'profiles', section: 'sw.sec.station', label: 'sw.tab.profiles', icon: '☰', render: () => <RadioTab part="profiles" /> },
  { id: 'rotator', section: 'sw.sec.station', label: 'sw.tab.rotator', icon: '⊕', render: () => <RotatorTab /> },
  { id: 'sdr', section: 'sw.sec.station', label: 'sw.tab.sdr', icon: '⌁', render: () => <SdrTab /> },
  { id: 'audio', section: 'sw.sec.station', label: 'sw.tab.audio', icon: '∿', render: () => <AudioTab /> },
  { id: 'freq', section: 'sw.sec.station', label: 'sw.tab.freq', icon: '∼', render: () => <FrequenciesTab /> },
  { id: 'bands', section: 'sw.sec.station', label: 'tab.bands', icon: '▦', render: () => <BandsTab /> },
  { id: 'macros', section: 'sw.sec.operate', label: 'sw.tab.macros', icon: '≡', render: () => <MacrosTab /> },
  { id: 'sequence', section: 'sw.sec.operate', label: 'tab.sequence', icon: '↻', render: () => <SequenceTab /> },
  { id: 'autocq', section: 'sw.sec.operate', label: 'sw.tab.autocq', icon: '◷', render: () => <ScheduleTab /> },
  { id: 'automation', section: 'sw.sec.operate', label: 'nav.automation', icon: '⚡', render: () => <AutomationTab /> },
  { id: 'filters', section: 'sw.sec.operate', label: 'sw.tab.filters', icon: '▽', render: () => <FiltersTab /> },
  { id: 'cqnews', section: 'sw.sec.operate', label: 'panel.cq', icon: '◉', render: () => <CqNewsTab /> },
  { id: 'wishlist', section: 'sw.sec.operate', label: 'tab.wishlist', icon: '★', render: () => <WishlistTab /> },
  { id: 'dxcc', section: 'sw.sec.operate', label: 'tab.dxcc', icon: '◎', render: () => <DxccTab /> },
  { id: 'aspetto', section: 'sw.sec.look', label: 'sw.tab.appearance', icon: 'Aa', render: () => <AppearanceTab /> },
  { id: 'colors', section: 'sw.sec.look', label: 'sw.tab.colors', icon: '◐', render: () => <NotificationsTab part="colors" /> },
  { id: 'alerts', section: 'sw.sec.look', label: 'sw.tab.alerts', icon: '♫', render: () => <NotificationsTab part="alerts" /> },
  { id: 'waterfall', section: 'sw.sec.look', label: 'tab.waterfall', icon: '▥', render: () => <WaterfallTab /> },
  { id: 'windows', section: 'sw.sec.look', label: 'tab.windows', icon: '▢', render: () => <WindowsTab /> },
  { id: 'clock', section: 'sw.sec.look', label: 'tab.clock', icon: '◔', render: () => <ClockTab /> },
  { id: 'logbooks', section: 'sw.sec.log', label: 'sw.tab.logbooks', icon: '▤', render: () => <LogServicesTab /> },
  { id: 'files', section: 'sw.sec.log', label: 'sw.tab.files', icon: '▣', render: () => <LogTransfer /> },
  { id: 'reporting', section: 'sw.sec.log', label: 'sw.tab.reporting', icon: '⇄', render: () => <ReportsTab /> },
  { id: 'online', section: 'sw.sec.log', label: 'tab.online', icon: '☁', render: () => <OnlineTab /> },
  { id: 'cluster', section: 'sw.sec.log', label: 'tab.cluster', icon: '✦', render: () => <ClusterTab /> },
  { id: 'phone', section: 'sw.sec.access', label: 'sw.tab.phone', icon: '☎', render: () => <AccessTab /> },
  { id: 'agent', section: 'sw.sec.access', label: 'agent.tab', icon: '◳', render: () => <AgentTab /> },
  { id: 'support', section: 'sw.sec.access', label: 'tab.support', icon: '✚', render: () => <SupportTab /> },
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
  /** Bozze di schede che non sono voci del motore (le regole delle Automazioni). */
  const [drafts, setDrafts] = useState<Record<string, unknown>>({});
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
  // i collegamenti dentro le schede («Apri Audio» nel riquadro della radio) chiedono di cambiare scheda con un evento
  useEffect(() => {
    const onTab = (e: Event) => {
      const want = (e as CustomEvent<string>).detail;
      if (!TABS.some((x) => x.id === want)) return;
      e.preventDefault();
      setId(want);
      rememberTab(browserStorage(), want);
    };
    window.addEventListener(SETTINGS_TAB_EVENT, onTab);
    return () => window.removeEventListener(SETTINGS_TAB_EVENT, onTab);
  }, []);
  // nel riquadro un nuovo indirizzo `#/impostazioni/<scheda>` con la finestra gia' aperta porta su quella scheda
  useEffect(() => {
    if (initialTab && TABS.some((x) => x.id === initialTab)) setId(initialTab);
  }, [initialTab]);
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
      clear: () => {
        setChanges({});
        setDrafts({});
      },
      drafts,
      setDraftOf(key, value) {
        setDrafts((previous) => {
          const next = { ...previous };
          if (value === undefined) delete next[key];
          else next[key] = value;
          return next;
        });
      },
    }),
    [changes, error, drafts],
  );
  const count = Object.keys(changes).length + Object.keys(drafts).length;

  /** Manda la bozza al motore; vero se e' andata. */
  const apply = async (): Promise<boolean> => {
    if (count === 0) return true;
    setBusy(true);
    try {
      const slow = Object.keys(changes).some((k) => settings.data?.items[k]?.apply === 'restart');
      if (Object.keys(changes).length > 0) client.setQueryData(['engine-settings'], await putEngineSettings(changes));
      const rules = drafts[AUTOMATION_DRAFT] as AutomationRules | undefined;
      if (rules) client.setQueryData(['automation'], await putAutomation({ rules }));
      setChanges({});
      setDrafts({});
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
  // «Salva» salva e lascia aperta la finestra (si continua con le altre schede); si chiude con la X o con «Chiudi»
  const save = async () => {
    // molte voci si salvano da sole appena cambiate: senza modifiche in sospeso si dice che e' tutto salvato
    if (count === 0) {
      setNote(t('sw.allSaved'));
      return;
    }
    await apply();
  };
  const discard = () => {
    usePrefs.getState().set(startPrefs.current);
    setChanges({});
    setDrafts({});
    leave();
  };
  // chiudendo con modifiche non salvate si chiede prima se scartarle
  const cancel = () => {
    if (count === 0) {
      leave();
      return;
    }
    void askConfirm(t('sw.discardAsk', { n: count })).then((yes) => {
      if (yes) discard();
    });
  };
  cancelRef.current = cancel;

  const testCat = async () => {
    setNote('…');
    try {
      // il collegamento parte in sottofondo: si aspetta l'esito (collegato o errore) fino a 15 secondi, invece di leggere subito «in corso»
      let snap = await connectCat();
      for (let i = 0; i < 30 && snap.status.state !== 'connected' && snap.status.state !== 'error'; i++) {
        await new Promise((r) => setTimeout(r, 500));
        snap = await getCat();
      }
      setNote(snap.status.state === 'connected' ? t('sw.catOk') : `${t('sw.catFail')} ${snap.status.message}`);
    } catch {
      setNote(t('sw.catFail'));
    }
  };
  const testPtt = async () => {
    if (!(await askConfirm(t('sw.pttConfirm')))) return;
    try {
      // il Tune parte all'inizio del prossimo periodo (fino a 15 s): si aspetta che la radio sia davvero in trasmissione, poi un secondo e si spegne
      await postTune(true);
      setNote(t('sw.pttWait'));
      let on = false;
      for (let i = 0; i < 40 && !on; i++) {
        await new Promise((r) => setTimeout(r, 500));
        on = useStation.getState().status?.transmitting === true;
      }
      if (on) await new Promise((r) => setTimeout(r, 1000));
      await postTune(false);
      setNote(on ? t('sw.pttDone') : t('sw.pttFail'));
    } catch {
      await postTune(false).catch(() => undefined);
      setNote(t('sw.pttFail'));
    }
  };

  const shownTabs = TABS;
  const tab = shownTabs.find((x) => x.id === id) ?? shownTabs[0]!;
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
          {shownTabs.map((x, i) => (
            <Fragment key={x.id}>
              {x.section && x.section !== shownTabs[i - 1]?.section ? <span className="sw-nav-sep">{t(x.section)}</span> : null}
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
          {t('sw.close')}
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
