import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect } from 'react';
import { checkSession } from './api/client';
import { openStream } from './api/stream';
import { Footer } from './components/Footer';
import { Logo } from './components/Logo';
import { LanguageSelect } from './components/LanguageSelect';
import { useT } from './i18n';
import { ThemeToggle } from './components/ThemeToggle';
import { UpdateBar } from './components/UpdateBar';
import { WantedBar } from './components/WantedBar';
import { applyColors, readColors } from './lib/colors';
import { useEscapeStop } from './lib/escapeStop';
import { rememberRoute, rememberedTab } from './lib/lastPage';
import { useRoute } from './lib/route';
import { browserStorage } from './lib/theme';
import { emitSpectrum } from './lib/spectrumFeed';
import { useStation } from './state/station';
import { Automation } from './pages/Automation';
import { Settings } from './pages/Settings';
import { Live } from './pages/Live';
import { HistoryPage } from './pages/HistoryPage';
import { MapPage } from './pages/MapPage';
import { PropagationPage } from './pages/PropagationPage';
import { Pair } from './pages/Pair';
import { Stats } from './pages/Stats';
import { Wiki } from './pages/Wiki';

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
  const session = useQuery({ queryKey: ['session'], queryFn: checkSession, retry: false });

  // Il flusso dei dati sta qui e non nella pagina della stazione: cambiando pagina (per esempio sulla mappa) i messaggi
  // continuano ad arrivare e non si perde nulla.
  const paired = !!session.data;
  useEffect(() => {
    if (!paired) return;
    const { apply, setConnection } = useStation.getState();
    const stream = openStream({ onEvent: apply, onSpectrum: (frame) => emitSpectrum(frame.bins), onState: setConnection });
    return () => stream.close();
  }, [paired]);

  const onPaired = () => {
    location.hash = '#/';
    void queryClient.invalidateQueries({ queryKey: ['session'] });
  };

  let content;
  if (route.name === 'pair') {
    content = <Pair initialCode={route.code} autoSubmit={route.code.length === 8} onPaired={onPaired} />;
  } else if (session.isPending) {
    content = <p className="muted">{t('app.verifying')}</p>;
  } else if (session.isError) {
    content = (
      <div className="notice">
        <p>{t('app.noCore')}</p>
        <button type="button" onClick={() => session.refetch()}>
          {t('app.retry')}
        </button>
      </div>
    );
  } else if (!session.data) {
    content = <Pair onPaired={onPaired} />;
  } else if (route.name === 'settings') {
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
  } else {
    content = <Live />;
  }

  return (
    <div className="app-shell">
      <header className="app-header">
        <a className="app-brand" href="#/">AURORA</a>
        <nav className="app-nav" aria-label={t('nav.pages')}>
          <a className={`glass${route.name === 'live' ? ' glass--on' : ''}`} aria-current={route.name === 'live' ? 'page' : undefined} href="#/">{t('nav.station')}</a>
          <a className={`glass${route.name === 'automation' ? ' glass--on' : ''}`} aria-current={route.name === 'automation' ? 'page' : undefined} href="#/automazioni">{t('nav.automation')}</a>
          <a className={`glass${route.name === 'stats' ? ' glass--on' : ''}`} aria-current={route.name === 'stats' ? 'page' : undefined} href="#/statistiche">{t('nav.stats')}</a>
          <a className={`glass${route.name === 'map' ? ' glass--on' : ''}`} aria-current={route.name === 'map' ? 'page' : undefined} href="#/mappa">{t('nav.map')}</a>
          <a className={`glass${route.name === 'history' ? ' glass--on' : ''}`} aria-current={route.name === 'history' ? 'page' : undefined} href="#/storico">{t('nav.history')}</a>
          <a className={`glass${route.name === 'propagation' ? ' glass--on' : ''}`} aria-current={route.name === 'propagation' ? 'page' : undefined} href="#/propagazione">{t('nav.propagation')}</a>
          <a className={`glass${route.name === 'settings' ? ' glass--on' : ''}`} aria-current={route.name === 'settings' ? 'page' : undefined} href="#/impostazioni">{t('nav.settings')}</a>
          <a className={`glass${route.name === 'guide' ? ' glass--on' : ''}`} aria-current={route.name === 'guide' ? 'page' : undefined} href="#/guida">{t('nav.wiki')}</a>
        </nav>
        <LanguageSelect />
        <ThemeToggle />
        <a className="app-logo-link" href="#/" aria-label="Aurora"><Logo size={36} animated /></a>
      </header>
      {session.data ? <WantedBar /> : null}
      {session.data ? <UpdateBar /> : null}
      <main className="app-main">{content}</main>
      <Footer />
    </div>
  );
}
