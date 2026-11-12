import { useStation } from '../state/station';
import { askConfirm } from '../components/ConfirmHost';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import { connectCat, getCat, getHealth, postTune, putAutomation, putEngineSettings, putSstv } from '../api/client';
import type { AutomationRules } from '../api/automation';
import { useT, type Key } from '../i18n';
import { SettingsSearch } from '../settings/SettingsSearch';
import { usePrefs } from '../lib/prefs';
import { SETTINGS_TAB_EVENT } from '../lib/settingsWindow';
import { allPages, INFO_ID, isKnownNode, resolveNode, SETTINGS_TREE, type FamilyId } from '../lib/settingsTree';
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
import { SSTV_DRAFT, SstvTab, sstvDraftToSettings, type SstvDraft } from '../settings/SstvTab';
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
import { PersonalTab } from '../settings/PersonalTab';
import { ContestSettingsPage, DigiClusterPage, DigiMacrosPage, DigiScanPage, DigiModePage, SatSettingsPage, WsprPage } from '../settings/ModePages';
import '../styles/settings-window.css';

/** Una pagina dell'albero: il nome e cosa disegna. */
interface PageDef {
  id: string;
  label: Key;
  render(): ReactNode;
}

/**
 * Cosa disegna ogni pagina dell'albero (`lib/settingsTree.ts`). I componenti sono quelli delle vecchie schede, alcuni divisi con `part`:
 * nessuna voce sparisce e nessuna compare due volte (a parte le scorciatoie volute: la barra di WSPR in Live, i dialoghi delle pagine).
 */
const RENDER: Readonly<Record<string, () => ReactNode>> = {
  // Stazione
  personal: () => <PersonalTab />,
  radio: () => <RadioTab part="link" />,
  profiles: () => <RadioTab part="profiles" />,
  audio: () => <AudioTab />,
  rotator: () => <RotatorTab />,
  sdr: () => <SdrTab />,
  clock: () => <ClockTab />,
  // Modi › Comune a tutti i modi
  freq: () => <FrequenciesTab />,
  qso: () => <GeneralWindowTab part="qso" />,
  autocq: () => <ScheduleTab />,
  // Modi › FT8 · FT4
  ft8decode: () => <FiltersTab part="ft8decode" />,
  sequence: () => <SequenceTab part="qso" />,
  macros: () => <MacrosTab />,
  automation: () => <AutomationTab />,
  messages: () => (
    <div className="sw-page">
      <FiltersTab part="lists" />
      <CqNewsTab />
      <GeneralWindowTab part="screen" />
    </div>
  ),
  // Modi › gli altri modi del motore
  jt: () => <FiltersTab part="jtdecode" />,
  vhf: () => <FiltersTab part="vhfdecode" />,
  wspr: () => <WsprPage />,
  // Modi › PSK · RTTY · CW
  digi: () => <DigiModePage />,
  digimacros: () => <DigiMacrosPage />,
  digiscan: () => <DigiScanPage />,
  digicluster: () => <DigiClusterPage />,
  // Modi › SSTV
  sstv: () => <SstvTab part="rx" />,
  sstvtx: () => <SstvTab part="tx" />,
  sstvimages: () => <SstvTab part="images" />,
  // Modi › Satelliti
  satorbits: () => <SatSettingsPage cat="orbits" />,
  satmap: () => <SatSettingsPage cat="map" />,
  satradio: () => <SatSettingsPage cat="radio" />,
  contest: () => <ContestSettingsPage group="ft8" />,
  contestvhf: () => <ContestSettingsPage group="vhf" />,
  // Log, cluster e servizi
  logbooks: () => <LogServicesTab />,
  files: () => <LogTransfer />,
  reporting: () => <ReportsTab />,
  cluster: () => <ClusterTab />,
  online: () => <OnlineTab />,
  wishlist: () => <WishlistTab />,
  dxcc: () => <DxccTab />,
  // Aspetto e avvisi
  aspetto: () => <AppearanceTab />,
  colors: () => <NotificationsTab part="colors" />,
  alerts: () => <NotificationsTab part="alerts" />,
  waterfall: () => <WaterfallTab />,
  windows: () => <WindowsTab />,
  bands: () => <BandsTab />,
  // Accesso e assistenza
  phone: () => <AccessTab />,
  agent: () => <AgentTab />,
  updates: () => <GeneralWindowTab part="updates" />,
  support: () => <SupportTab />,
  [INFO_ID]: () => <AboutTab />,
};

/** Tutte le pagine, nell'ordine dell'albero e con Informazioni in fondo: servono alla ricerca. */
export const SETTINGS_PAGES: readonly PageDef[] = [
  ...allPages().map(({ page }) => ({ id: page.id, label: page.label, render: RENDER[page.id] ?? (() => null) })),
  { id: INFO_ID, label: 'sw.tab.info', render: RENDER[INFO_ID] ?? (() => null) },
];

/** Chiude la finestra: dal programma per Windows serve l'indirizzo speciale, `window.close()` da solo non basta. */
function closeWindow(): void {
  window.close();
  location.href = `${location.origin}/__chiudi-finestra`;
}

/**
 * Le Impostazioni in una finestra a parte (o nel riquadro sopra l'applicazione): a sinistra l'albero (famiglie colorate, sottofamiglie,
 * pagine, che si aprono a fisarmonica), a destra il percorso e la pagina scelta, in basso Prova CAT, Prova PTT, Chiudi e Salva. Senza una
 * pagina richiesta tutto e' chiuso e si vede Informazioni. Le voci del motore scrivono in una bozza unica (`SessionDraft`) e partono solo
 * con Salva. Sotto i 760 px l'albero e' un elenco a tutta larghezza: scelta una pagina, compare il contenuto con «‹ Indietro».
 */
export function SettingsWindow({ initialTab, embedded = false, onClose }: { initialTab?: string; /** Dentro l'applicazione (riquadro sopra la pagina) e non in una finestra a parte. */ embedded?: boolean; onClose?(): void }) {
  const t = useT();
  const client = useQueryClient();
  const settings = useSettings();
  const start = resolveNode(initialTab);
  /** La pagina mostrata. */
  const [id, setId] = useState(start.page);
  /** La famiglia e la sottofamiglia aperte (una per livello: aprendone una le sorelle si chiudono). */
  const [openFamily, setOpenFamily] = useState<FamilyId | null>(start.family?.id ?? null);
  const [openSub, setOpenSub] = useState<string | null>(start.sub?.id ?? null);
  /** Sul telefono: l'elenco dell'albero o la pagina scelta. */
  const [view, setView] = useState<'tree' | 'page'>(initialTab && isKnownNode(initialTab) ? 'page' : 'tree');
  const health = useQuery({ queryKey: ['health'], queryFn: getHealth, staleTime: 60_000 });
  /** Porta su un nodo (pagina, sottofamiglia, famiglia o id vecchio) aprendo i rami fino a li'. */
  const go = (node: string) => {
    const path = resolveNode(node);
    setId(path.page);
    setOpenFamily(path.family?.id ?? null);
    setOpenSub(path.sub?.id ?? null);
    setView('page');
  };
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
      if (!isKnownNode(want)) return;
      e.preventDefault();
      go(want);
    };
    window.addEventListener(SETTINGS_TAB_EVENT, onTab);
    return () => window.removeEventListener(SETTINGS_TAB_EVENT, onTab);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  // nel riquadro un nuovo indirizzo `#/impostazioni/<scheda>` con la finestra gia' aperta porta su quella scheda
  useEffect(() => {
    if (initialTab && isKnownNode(initialTab)) go(initialTab);
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
    // la bozza SSTV si controlla prima di mandare qualcosa: un numero fuori misura ferma tutto
    const sstvDraft = drafts[SSTV_DRAFT] as SstvDraft | undefined;
    const sstvSettings = sstvDraft ? sstvDraftToSettings(sstvDraft) : null;
    if (sstvDraft && !sstvSettings) {
      setNote(t('sstv.set.invalid'));
      return false;
    }
    setBusy(true);
    try {
      const slow = Object.keys(changes).some((k) => settings.data?.items[k]?.apply === 'restart');
      if (Object.keys(changes).length > 0) client.setQueryData(['engine-settings'], await putEngineSettings(changes));
      const rules = drafts[AUTOMATION_DRAFT] as AutomationRules | undefined;
      if (rules) client.setQueryData(['automation'], await putAutomation({ rules }));
      if (sstvSettings) client.setQueryData(['sstv'], await putSstv(sstvSettings));
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

  const pageDef = SETTINGS_PAGES.find((x) => x.id === id) ?? SETTINGS_PAGES[SETTINGS_PAGES.length - 1]!;
  const where = resolveNode(pageDef.id);
  const accent = where.family?.color ?? '#8e7bff';
  const version = health.data?.version ?? '';
  /** Aprire una famiglia chiude le altre; aprirla di nuovo la chiude. La pagina mostrata non cambia. */
  const toggleFamily = (fam: FamilyId) => {
    setOpenFamily((now) => (now === fam ? null : fam));
    setOpenSub(null);
  };
  /** Lo stesso per le sottofamiglie: una sola aperta. */
  const toggleSub = (sub: string) => setOpenSub((now) => (now === sub ? null : sub));
  const pageButton = (pid: string, label: Key, level: 2 | 3) => (
    <button
      key={pid}
      type="button"
      id={`swtab-${pid}`}
      className={`sw-l${level}${pid === pageDef.id ? ' sw-on' : ''}`}
      aria-current={pid === pageDef.id ? 'page' : undefined}
      onClick={() => go(pid)}
    >
      {t(label)}
    </button>
  );
  const body = (
    <div className={`sw-window${embedded ? ' sw-window--in' : ''}`} role="dialog" aria-label={t('set.title')} style={{ '--sw-fam': accent } as CSSProperties}>
      <header className="sw-top" data-tauri-drag-region>
        <strong>{t('set.title')}</strong>
        {/* cerca ogni singola opzione in tutte le pagine: un clic apre la pagina (con i rami aperti) e porta all'opzione */}
        <SettingsSearch
          tabs={SETTINGS_PAGES}
          wrap={(node) => <SessionDraft.Provider value={draft}>{node}</SessionDraft.Provider>}
          onPick={(tabId, text) => {
            go(tabId);
            // la pagina ha bisogno di un momento per disegnarsi: poi si porta in vista l'opzione e la si illumina
            window.setTimeout(() => {
              const panel = document.getElementById('sw-panel');
              if (!panel) return;
              const norm = (v: string) => v.replace(/\s+/g, ' ').trim();
              const target = Array.from(panel.querySelectorAll<HTMLElement>('label, legend, h2, h3, h4, summary, th, [data-opt], input[aria-label], select[aria-label]')).find(
                (el) => norm(el.getAttribute('aria-label') ?? el.textContent ?? '') === text,
              );
              if (!target) return;
              target.scrollIntoView({ block: 'center', behavior: 'smooth' });
              target.classList.add('sw-flash');
              window.setTimeout(() => target.classList.remove('sw-flash'), 2200);
            }, 250);
          }}
        />
        <button type="button" className="sw-x" aria-label={t('sw.close')} title={t('sw.close')} onClick={cancel}>
          ✕
        </button>
      </header>
      <div className="sw-main" data-view={view}>
        <nav className="sw-tree" aria-label={t('set.title')}>
          {SETTINGS_TREE.map((fam) => {
            const open = openFamily === fam.id;
            return (
              <div key={fam.id} className={`sw-fam-wrap${open ? ' sw-open' : ''}`} style={{ '--fam': fam.color } as CSSProperties}>
                <button type="button" id={`swfam-${fam.id}`} className="sw-fam" aria-expanded={open} onClick={() => toggleFamily(fam.id)}>
                  <span className="sw-bar" aria-hidden="true" />
                  {t(fam.label)}
                  <span className="sw-chev" aria-hidden="true">
                    {open ? '⌄' : '›'}
                  </span>
                </button>
                {open
                  ? fam.items.map((item) => {
                      if (item.kind === 'page') return pageButton(item.id, item.label, 2);
                      const subOpen = openSub === item.id;
                      return (
                        <div key={item.id} className="sw-sub">
                          <button type="button" id={`swsub-${item.id}`} className={`sw-l2 sw-l2--sub${subOpen ? ' sw-open' : ''}`} aria-expanded={subOpen} onClick={() => toggleSub(item.id)}>
                            {t(item.label)}
                            <span className="sw-chev" aria-hidden="true">
                              {subOpen ? '⌄' : '›'}
                            </span>
                          </button>
                          {subOpen ? item.pages.map((pg) => pageButton(pg.id, pg.label, 3)) : null}
                        </div>
                      );
                    })
                  : null}
              </div>
            );
          })}
          <span className="sw-tree-gap" aria-hidden="true" />
          {/* Informazioni, staccata in fondo, con la versione */}
          <button
            type="button"
            id={`swtab-${INFO_ID}`}
            className={`sw-fam sw-fam--info${pageDef.id === INFO_ID ? ' sw-on' : ''}`}
            aria-current={pageDef.id === INFO_ID ? 'page' : undefined}
            onClick={() => go(INFO_ID)}
          >
            <span className="sw-bar" aria-hidden="true" />
            {t('sw.tab.info')}
            {version ? <span className="sw-ver data">{version}</span> : null}
          </button>
        </nav>
        <div className="sw-content">
          <div className="sw-crumbs">
            <button type="button" className="sw-back" onClick={() => setView('tree')}>
              ‹ {t('sw.back')}
            </button>
            <ol aria-label={t('sw.path')}>
              {where.family ? <li className="sw-crumb-fam">{t(where.family.label)}</li> : null}
              {where.sub ? <li>{t(where.sub.label)}</li> : null}
              <li aria-current="page">{t(pageDef.label)}</li>
            </ol>
          </div>
          <section className="sw-panel settings settings-layout" id="sw-panel" aria-label={t(pageDef.label)}>
            <SessionDraft.Provider value={draft}>{pageDef.render()}</SessionDraft.Provider>
          </section>
        </div>
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
