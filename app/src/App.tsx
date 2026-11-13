import { ConfirmHost } from './components/ConfirmHost';
import { DxMapPage } from './pages/DxMapPage';
import { attachAll, dropClosed, reopenAll, useGroups } from './lib/detached';
import './styles/solo.css';
import { SoloWindow } from './components/SoloWindow';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import { checkSession } from './api/client';
import { openStream } from './api/stream';
import { Footer } from './components/Footer';
import { ModeNav } from './components/ModeNav';
import { useT } from './i18n';
import { useEngineOsSync } from './lib/engineOs';
import { UpdateBar } from './components/UpdateBar';
import { WantedBar } from './components/WantedBar';
import { COLORS_STORAGE_KEY, applyColors, readColors } from './lib/colors';
import { useEscapeStop } from './lib/escapeStop';
import { rememberRoute } from './lib/lastPage';
import { addressFromName, findOtherAddress, rememberAddresses } from './lib/knownAddresses';
import { REMOTE_HOST } from './lib/phone';
import { followNavigateRequests } from './lib/sdr';
import { useRoute } from './lib/route';
import { browserStorage } from './lib/theme';
import { emitSpectrum } from './lib/spectrumFeed';
import { FoxPage } from './pages/FoxPage';
import { ContestPage } from './pages/ContestPage';
import { PageGuard } from './components/PageGuard';
import { QmapPage } from './pages/QmapPage';
import { useStation } from './state/station';
import { usePrefs } from './lib/prefs';
import { SettingsWindow } from './pages/SettingsWindow';
import { Live } from './pages/Live';
import { HistoryPage } from './pages/HistoryPage';
import { MapPage } from './pages/MapPage';
import { PropagationPage } from './pages/PropagationPage';
import { RotatorPage } from './pages/RotatorPage';
import { StationRotor } from './components/rotor/StationRotor';
import { SstvPage } from './pages/SstvPage';
import { DigiPage } from './pages/DigiPage';
import { SatPage } from './pages/SatPage';
import { Pair } from './pages/Pair';
import { Stats } from './pages/Stats';
import { ExportPage } from './pages/ExportPage';
import { News } from './pages/News';
import { Wiki } from './pages/Wiki';
import { Toaster } from './components/Toaster';
import { ReportsNotice } from './components/ReportsNotice';
import { LogPrompt } from './components/LogPrompt';
import { FirstRun } from './components/FirstRun';
import { UpdatePoster } from './components/UpdatePoster';
import { OwnerMessages } from './components/OwnerMessages';
import { OwnerMessagesTab } from './settings/OwnerMessagesTab';
import { CrashPrompt } from './settings/ReportsBox';

/**
 * Telefono, tablet o un altro PC sulla rete sono un telecomando: le Impostazioni si cambiano solo dalla finestra di Aurora
 * (Tauri) o da un browser sullo stesso computer.
 */
const REMOTE = REMOTE_HOST;
/** Telefono o tablet collegato da remoto: una sola pagina, la Stazione come telecomando. */
const REMOTE_TOUCH = REMOTE && typeof window !== 'undefined' && window.matchMedia?.('(pointer: coarse)').matches === true;

export function App() {
  // i testi che nominano Windows cambiano se il motore gira su un altro sistema
  useEngineOsSync();
  const t = useT();
  const showQmap = usePrefs((p) => p.prefs.showQmap);
  const route = useRoute();
  useEscapeStop();
  const queryClient = useQueryClient();
  useEffect(() => {
    rememberRoute(browserStorage(), route);
  }, [route]);
  // chiudendo le Impostazioni si torna alla pagina da cui le si e' aperte (PSK, Satelliti...), non sempre alla Stazione
  const beforeSettings = useRef('#/');
  useEffect(() => {
    if (route.name !== 'settings') beforeSettings.current = location.hash || '#/';
  }, [route]);
  useEffect(() => {
    applyColors(document.documentElement, readColors(browserStorage()));
    // i colori cambiati in un'altra finestra (Impostazioni nella finestra principale, mappa staccata): si rileggono e si applicano anche qui
    const onStorage = (e: StorageEvent) => {
      if (e.key !== null && e.key !== COLORS_STORAGE_KEY) return;
      applyColors(document.documentElement, readColors(browserStorage()));
    };
    window.addEventListener('storage', onStorage);
    return () => window.removeEventListener('storage', onStorage);
  }, []);
  // finche' il dispositivo non e' associato si riprova: se dal PC si accende l'accesso senza codice, il telefono entra da solo
  const session = useQuery({ queryKey: ['session'], queryFn: checkSession, retry: false, refetchInterval: (q) => (q.state.data === false ? 3000 : false) });
  // appena entra si ricarica la pagina: cosi' prende dal server le impostazioni di Aurora (bande, macro, filtri, tema, lingua)
  const wasUnpaired = useRef(false);
  useEffect(() => {
    if (session.data === false) wasUnpaired.current = true;
    else if (session.data === true && wasUnpaired.current) {
      wasUnpaired.current = false;
      if (location.hash.startsWith('#/pair') || location.hash.startsWith('#/associa')) history.replaceState(null, '', '#/');
      location.reload();
    }
  }, [session.data]);

  // Il flusso dei dati sta qui e non nella pagina della stazione: cambiando pagina (per esempio sulla mappa) i messaggi
  // continuano ad arrivare e non si perde nulla.
  const paired = !!session.data;
  useEffect(() => {
    if (paired) rememberAddresses(browserStorage(), [window.location.origin]);
  }, [paired]);
  useEffect(() => {
    if (!paired) return;
    const { apply, setConnection } = useStation.getState();
    const stream = openStream({ onEvent: apply, onSpectrum: (frame) => emitSpectrum(frame.bins, frame.fMinHz, frame.fMaxHz), onState: setConnection });
    return () => stream.close();
  }, [paired]);

  // dopo l'associazione il telefono si ricorda questo indirizzo e quello fisso del QR (aurora.local)
  const pairAlt = route.name === 'pair' ? route.alt : undefined;
  const onPaired = () => {
    const alt = pairAlt ? addressFromName(pairAlt, window.location) : null;
    rememberAddresses(browserStorage(), alt ? [window.location.origin, alt] : [window.location.origin]);
    location.hash = '#/';
    void queryClient.invalidateQueries({ queryKey: ['session'] });
  };

  // finestre a parte: all'avvio della finestra principale si riaprono dove erano; quelle chiuse con la X rimettono i pannelli al loro posto
  const groups = useGroups();
  const isSolo = route.name === 'window';
  useEffect(() => {
    if (!isSolo && !REMOTE && session.data === true) reopenAll();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session.data === true]);
  useEffect(() => {
    if (isSolo || !groups.some((g) => !g.open)) return;
    const timer = window.setTimeout(dropClosed, 800);
    return () => window.clearTimeout(timer);
  }, [groups, isSolo]);

  // la finestra principale apre le pagine chieste dalle finestre a parte (per esempio QMAP o la Guida dalla scheda SDR delle Impostazioni)
  useEffect(() => (isSolo ? undefined : followNavigateRequests()), [isSolo]);

  // le Impostazioni sono un riquadro al centro sopra l'applicazione, che dietro si oscura e non si puo' usare
  const settingsOpen = route.name === 'settings' && !REMOTE && session.data === true;

  if (route.name === 'window' && route.id === 'impostazioni' && session.data === true) {
    return <SettingsWindow initialTab={new URLSearchParams(location.search).get('tab') ?? undefined} />;
  }
  // finestra a parte (secondo schermo): solo la pagina, senza menu, barre e finestre di avviso
  if (route.name === 'window' && session.data === true) {
    return <SoloWindow id={route.id} />;
  }

  const LIVE = <Live />;
  let content: React.ReactNode;
  if (route.name === 'pair' && session.data !== true) {
    content = <Pair initialCode={route.code} autoSubmit={route.code.length === 8} onPaired={onPaired} />;
  } else if (session.isPending) {
    content = <p className="muted">{t('app.verifying')}</p>;
  } else if (REMOTE_TOUCH && session.data === true) {
    content = LIVE;
  } else if (session.isError) {
    content = (
      <div className="notice">
        <p>{t('app.noCore')}</p>
        {REMOTE ? <OtherAddress /> : null}
        <button type="button" onClick={() => session.refetch()}>
          {t('app.retry')}
        </button>
      </div>
    );
  } else if (!session.data) {
    content = <Pair onPaired={onPaired} />;
  } else if (route.name === 'settings' && !REMOTE) {
    // le Impostazioni stanno nel riquadro (qui sotto): dietro resta la Stazione
    content = LIVE;
  } else if (route.name === 'stats') {
    content = <Stats />;
  } else if (route.name === 'map') {
    content = <MapPage />;
  } else if (route.name === 'dxmap') {
    content = <DxMapPage />;
  } else if (route.name === 'history') {
    content = <HistoryPage />;
  } else if (route.name === 'propagation') {
    content = <PropagationPage />;
  } else if (route.name === 'rotator') {
    content = <RotatorPage />;
  } else if (route.name === 'sstv') {
    content = <SstvPage />;
  } else if (route.name === 'digi') {
    content = <DigiPage />;
  } else if (route.name === 'sat') {
    content = <SatPage />;
  } else if (route.name === 'guide') {
    content = <Wiki />;
  } else if (route.name === 'fox') {
    content = <FoxPage />;
  } else if (route.name === 'contest') {
    content = <ContestPage />;
  } else if (route.name === 'qmap') {
    content = <QmapPage />;
  } else if (route.name === 'ownerMessages') {
    content = (
      <div className="owner-page">
        <h1>{t('own.tab')}</h1>
        <OwnerMessagesTab />
      </div>
    );
  } else if (route.name === 'news') {
    content = <News />;
  } else if (route.name === 'export') {
    content = <ExportPage />;
  } else {
    content = LIVE;
  }

  return (
    <>
    <div className="app-shell" inert={settingsOpen ? true : undefined}>
      {REMOTE_TOUCH ? null : (
      <header className="app-header">
        <a className="app-brand" href="#/">
          AURORA
          <span className="app-brand-by">by IU3CYV</span>
        </a>
        <nav className="app-nav" aria-label={t('nav.pages')}>
          <ModeNav route={route} showQmap={showQmap} remote={REMOTE} />
          {/* solo nella prova aperta nel browser: mai nel programma installato (finestra di Aurora), nemmeno con la grafica di prova */}
          {import.meta.env.VITE_AURORA_PROVA === '1' && !('__TAURI_INTERNALS__' in window) ? (
            <span className="app-test-badge" title="Grafica di prova compilata sul PC: non e' la versione pubblicata">
              PROVA LOCALE
            </span>
          ) : null}
        </nav>
        {/* lingua, volume, tema: piccoli, subito dopo il menu, staccati da un pallino colorato */}
        <span className="app-tools">
          <span className="app-tools-dot" aria-hidden="true" />
          {groups.length > 0 ? (
            <button type="button" className="solo-btn app-attach-all" title={t('win.attachAll')} aria-label={t('win.attachAll')} onClick={attachAll}>
              ⤓
            </button>
          ) : null}
          {session.data ? <UpdateBar /> : null}
          {/* avvisi e tema stanno nella barra in fondo (menu nuovo); la lingua si sceglie in Impostazioni (Generali) */}
        </span>
      </header>
      )}
      {session.data ? <PageGuard resetKey={route.name}><WantedBar /></PageGuard> : null}
      <main className="app-main">
        <PageGuard resetKey={route.name}>{content}</PageGuard>
      </main>
      {/* il rotore sopra la Stazione in tutti i modi del motore (FT8, FT4, JT65, Q65, ...), Fox e Contest: la stessa scheda di PSK/RTTY/CW;
          con le Impostazioni aperte no (starebbe sopra il loro riquadro) */}
      {session.data === true && !REMOTE_TOUCH && !settingsOpen && (content === LIVE || route.name === 'fox' || route.name === 'contest') ? <PageGuard resetKey={route.name}><StationRotor /></PageGuard> : null}
      {/* la barra in fondo, la barra dei desiderati e il rotore stanno fuori dalla pagina: un loro errore non deve svuotare tutta la finestra */}
      {REMOTE_TOUCH ? null : <PageGuard resetKey={route.name}><Footer page={route.name} /></PageGuard>}
      <ConfirmHost />
      {session.data && !REMOTE ? <CrashPrompt /> : null}
      {session.data && !REMOTE ? <ReportsNotice /> : null}
      {session.data && !REMOTE ? <FirstRun /> : null}
      {session.data ? <LogPrompt /> : null}
      {/* i messaggi del proprietario (auguri, avvisi): sempre accesi, solo sul PC */}
      {session.data && !REMOTE ? <OwnerMessages /> : null}
    </div>
    {/* senza una pagina richiesta le Impostazioni si aprono su Informazioni, con l'albero tutto chiuso */}
    {settingsOpen ? <SettingsWindow embedded initialTab={route.name === 'settings' ? route.tab : undefined} onClose={() => { location.hash = beforeSettings.current; }} /> : null}
    {/* i messaggi brevi fuori da `.app-shell`: con le Impostazioni aperte (pagina sotto inert) si devono poter chiudere */}
    <Toaster />
    {/* fuori dalla pagina principale, dopo le Impostazioni: quando sono aperte la pagina sotto e' disattivata (inert) e la locandina
        dell'aggiornamento, che stava li' dentro, restava nascosta e non cliccabile */}
    {session.data && !REMOTE ? <UpdatePoster /> : null}
    </>
  );
}

/** Aurora non risponde a questo indirizzo: si provano quelli ricordati e, se uno risponde, si propone di passarci. */
function OtherAddress() {
  const t = useT();
  const [found, setFound] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    void findOtherAddress(browserStorage(), window.location.origin).then((o) => alive && setFound(o));
    return () => {
      alive = false;
    };
  }, []);
  if (!found) return null;
  return (
    <p>
      {t('app.moved')}{' '}
      <a className="glass" href={`${found}/#/`} title={t('tip.app.moved')}>
        {t('app.moveTo', { url: found.replace(/^https?:\/\//, '') })}
      </a>
    </p>
  );
}
