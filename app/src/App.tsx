import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect } from 'react';
import { checkSession } from './api/client';
import { Footer } from './components/Footer';
import { ThemeToggle } from './components/ThemeToggle';
import { applyColors, readColors } from './lib/colors';
import { useRoute } from './lib/route';
import { browserStorage } from './lib/theme';
import { Automation } from './pages/Automation';
import { Settings } from './pages/Settings';
import { Live } from './pages/Live';
import { Pair } from './pages/Pair';

export function App() {
  const route = useRoute();
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
    content = <p className="muted">Verifica del dispositivo…</p>;
  } else if (session.isError) {
    content = (
      <div className="notice">
        <p>Non riesco a contattare Aurora. Controlla che sia acceso sul PC e che la rete sia raggiungibile.</p>
        <button type="button" onClick={() => session.refetch()}>
          Riprova
        </button>
      </div>
    );
  } else if (!session.data) {
    content = <Pair onPaired={onPaired} />;
  } else if (route.name === 'settings') {
    content = <Settings />;
  } else if (route.name === 'automation') {
    content = <Automation />;
  } else {
    content = <Live />;
  }

  return (
    <div className="app-shell">
      <header className="app-header">
        <a className="app-brand" href="#/">AURORA</a>
        <nav className="app-nav" aria-label="Pagine">
          <a className={`glass${route.name === 'live' ? ' glass--on' : ''}`} href="#/">Stazione</a>
          <a className={`glass${route.name === 'automation' ? ' glass--on' : ''}`} href="#/automazioni">Automazioni</a>
          <a className={`glass${route.name === 'settings' ? ' glass--on' : ''}`} href="#/impostazioni">Impostazioni</a>
        </nav>
        <ThemeToggle />
      </header>
      <main className="app-main">{content}</main>
      <Footer />
    </div>
  );
}
