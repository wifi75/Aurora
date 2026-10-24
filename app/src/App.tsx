import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect } from 'react';
import { checkSession } from './api/client';
import { Footer } from './components/Footer';
import { LanguageSelect } from './components/LanguageSelect';
import { useT } from './i18n';
import { ThemeToggle } from './components/ThemeToggle';
import { applyColors, readColors } from './lib/colors';
import { useEscapeStop } from './lib/escapeStop';
import { useRoute } from './lib/route';
import { browserStorage } from './lib/theme';
import { Automation } from './pages/Automation';
import { Settings } from './pages/Settings';
import { Live } from './pages/Live';
import { Pair } from './pages/Pair';
import { Stats } from './pages/Stats';
import { Wiki } from './pages/Wiki';

export function App() {
  const t = useT();
  const route = useRoute();
  useEscapeStop();
  const queryClient = useQueryClient();
  useEffect(() => {
    applyColors(document.documentElement, readColors(browserStorage()));
  }, []);
  const session = useQuery({ queryKey: ['session'], queryFn: checkSession, retry: false });

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
    content = <Settings />;
  } else if (route.name === 'automation') {
    content = <Automation />;
  } else if (route.name === 'stats') {
    content = <Stats />;
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
          <a className={`glass${route.name === 'live' ? ' glass--on' : ''}`} href="#/">{t('nav.station')}</a>
          <a className={`glass${route.name === 'automation' ? ' glass--on' : ''}`} href="#/automazioni">{t('nav.automation')}</a>
          <a className={`glass${route.name === 'stats' ? ' glass--on' : ''}`} href="#/statistiche">{t('nav.stats')}</a>
          <a className={`glass${route.name === 'settings' ? ' glass--on' : ''}`} href="#/impostazioni">{t('nav.settings')}</a>
          <a className={`glass${route.name === 'guide' ? ' glass--on' : ''}`} href="#/guida">{t('nav.wiki')}</a>
        </nav>
        <LanguageSelect />
        <ThemeToggle />
      </header>
      <main className="app-main">{content}</main>
      <Footer />
    </div>
  );
}
