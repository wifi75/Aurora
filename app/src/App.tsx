import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import { checkSession } from './api/client';
import { openStream } from './api/stream';
import { Footer } from './components/Footer';
import { Logo } from './components/Logo';
import { LanguageSelect } from './components/LanguageSelect';
import { HeaderAlertSwitch } from './components/AlertSwitch';
import { useT } from './i18n';
import { ThemeToggle } from './components/ThemeToggle';
import { UpdateBar } from './components/UpdateBar';
import { WantedBar } from './components/WantedBar';
import { applyColors, readColors } from './lib/colors';
import { useEscapeStop } from './lib/escapeStop';
import { rememberRoute, rememberedTab } from './lib/lastPage';
import { addressFromName, findOtherAddress, rememberAddresses } from './lib/knownAddresses';
import { REMOTE_HOST } from './lib/phone';
import { useRoute } from './lib/route';
import { browserStorage } from './lib/theme';
import { emitSpectrum } from './lib/spectrumFeed';
import { FoxPage } from './pages/FoxPage';
import { useStation } from './state/station';
import { Automation } from './pages/Automation';
import { Settings } from './pages/Settings';
import { Live } from './pages/Live';
import { HistoryPage } from './pages/HistoryPage';
import { MapPage } from './pages/MapPage';
import { PropagationPage } from './pages/PropagationPage';
import { Pair } from './pages/Pair';
import { Stats } from './pages/Stats';
import { ExportPage } from './pages/ExportPage';
import { News } from './pages/News';
import { Wiki } from './pages/Wiki';
import { Toaster } from './components/Toaster';
import { CrashPrompt } from './settings/ReportsBox';

/**
 * Telefono, tablet o un altro PC sulla rete sono un telecomando: le Impostazioni si cambiano solo dalla finestra di Aurora
 * (Tauri) o da un browser sullo stesso computer.
 */
const REMOTE = REMOTE_HOST;
/** Telefono o tablet collegato da remoto: una sola pagina, la Stazione come telecomando. */
const REMOTE_TOUCH = REMOTE && typeof window !== 'undefined' && window.matchMedia?.('(pointer: coarse)').matches === true;

export function App() {
  const t = useT();
  const route = useRoute();
  useEscapeStop();
  const queryClient = useQueryClient();
  useEffect(() => {
    rememberRoute(browserStorage(), route);
  }, [route]);
  useEffect(() => {
    applyColors(document.documentElement, readColors(browserStorage()));
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

  let content;
  if (route.name === 'pair' && session.data !== true) {
    content = <Pair initialCode={route.code} autoSubmit={route.code.length === 8} onPaired={onPaired} />;
  } else if (session.isPending) {
    content = <p className="muted">{t('app.verifying')}</p>;
  } else if (REMOTE_TOUCH && session.data === true) {
    content = <Live />;
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
    content = <Settings key={route.tab ?? ''} initialTab={route.tab ?? rememberedTab(browserStorage())} />;
  } else if (route.name === 'automation') {
    content = <Automation />;
  } else if (route.name === 'stats') {
    content = <Stats />;
  } else if (route.name === 'map') {
    content = <MapPage />;
  } else if (route.name === 'history') {
    content = <HistoryPage />;
  } else if (route.name === 'propagation') {
    content = <PropagationPage />;
  } else if (route.name === 'guide') {
    content = <Wiki />;
  } else if (route.name === 'fox') {
    content = <FoxPage />;
  } else if (route.name === 'news') {
    content = <News />;
  } else if (route.name === 'export') {
    content = <ExportPage />;
  } else {
    content = <Live />;
  }

  return (
    <div className="app-shell">
      {REMOTE_TOUCH ? null : (
      <header className="app-header">
        <a className="app-brand" href="#/">
          AURORA
          <span className="app-brand-by">by IU3CYV</span>
        </a>
        <nav className="app-nav" aria-label={t('nav.pages')}>
          <a className={`glass${route.name === 'live' ? ' glass--on' : ''}`} aria-current={route.name === 'live' ? 'page' : undefined} href="#/">{t('nav.station')}</a>
          <a className={`glass${route.name === 'fox' ? ' glass--on' : ''}`} aria-current={route.name === 'fox' ? 'page' : undefined} href="#/fox" title={t('tip.nav.fox')}>{t('nav.fox')}</a>
          <a className={`glass${route.name === 'automation' ? ' glass--on' : ''}`} aria-current={route.name === 'automation' ? 'page' : undefined} href="#/automazioni">{t('nav.automation')}</a>
          <a className={`glass${route.name === 'stats' ? ' glass--on' : ''}`} aria-current={route.name === 'stats' ? 'page' : undefined} href="#/statistiche">{t('nav.stats')}</a>
          <a className={`glass${route.name === 'map' ? ' glass--on' : ''}`} aria-current={route.name === 'map' ? 'page' : undefined} href="#/mappa">{t('nav.map')}</a>
          {/* «Log»: tutti i QSO con filtri ed esportazione ADIF; lo storico dei messaggi ricevuti non e' piu' nel menu (scelta del proprietario) */}
          <a className={`glass${route.name === 'export' ? ' glass--on' : ''}`} aria-current={route.name === 'export' ? 'page' : undefined} href="#/esporta">{t('nav.log')}</a>
          <a className={`glass${route.name === 'propagation' ? ' glass--on' : ''}`} aria-current={route.name === 'propagation' ? 'page' : undefined} href="#/propagazione">{t('nav.propagation')}</a>
          {REMOTE ? null : (
            <a className={`glass${route.name === 'settings' ? ' glass--on' : ''}`} aria-current={route.name === 'settings' ? 'page' : undefined} href="#/impostazioni">{t('nav.settings')}</a>
          )}
          <a className={`glass${route.name === 'guide' ? ' glass--on' : ''}`} aria-current={route.name === 'guide' ? 'page' : undefined} href="#/guida">{t('nav.wiki')}</a>
          <a className={`glass${route.name === 'news' ? ' glass--on' : ''}`} aria-current={route.name === 'news' ? 'page' : undefined} href="#/novita">{t('nav.news')}</a>
        </nav>
        {session.data ? <UpdateBar /> : null}
        <LanguageSelect />
        <HeaderAlertSwitch />
        <ThemeToggle />
        <a className="app-logo-link" href="#/" aria-label="Aurora"><Logo size={36} animated /></a>
      </header>
      )}
      {session.data ? <WantedBar /> : null}
      <main className="app-main">{content}</main>
      {REMOTE_TOUCH ? null : <Footer />}
      <Toaster />
      {session.data && !REMOTE ? <CrashPrompt /> : null}
    </div>
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
