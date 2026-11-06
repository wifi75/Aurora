import '@fontsource/ibm-plex-sans/400.css';
import '@fontsource/ibm-plex-sans/500.css';
import '@fontsource/jetbrains-mono/400.css';
import '@fontsource/jetbrains-mono/500.css';
import 'flag-icons/css/flag-icons.min.css';
import './styles/tokens.css';
import './styles/base.css';
import './styles/components.css';
import './styles/glass.css';
import './styles/onair.css';
import './styles/radiodisplay.css';
import './styles/dxcc.css';
import './styles/settings-layout.css';
import './styles/news.css';
import './styles/bozze.css';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { watchFreshUi } from './lib/freshUi';
import { deviceKind, hydrate, keepSynced } from './lib/syncSettings';

// bozze grafiche della pagina principale: `?bozza=a|b|c` (vedi styles/bozze.css); senza il parametro non cambia niente
const draftStyle = new URLSearchParams(window.location.search).get('bozza');
if (draftStyle && /^[abc]$/.test(draftStyle)) document.documentElement.dataset.bozza = draftStyle;

const queryClient = new QueryClient();
const root = document.getElementById('root');

// le preferenze si portano dal server prima di disegnare (i moduli le leggono all'avvio) e poi si tengono allineate
async function start() {
  const kind = deviceKind(window.innerWidth);
  try {
    await hydrate(window.localStorage, kind);
    keepSynced(kind);
  } catch {
    // senza archivio del browser l'interfaccia parte con i valori predefiniti
  }
  // il menu del tasto destro del browser (Indietro, Ricarica, Ispeziona...) non serve: resta solo nei campi di testo, per copia e incolla
  document.addEventListener('contextmenu', (e) => {
    const target = e.target instanceof Element ? e.target : null;
    if (target?.closest('input, textarea, select, [contenteditable="true"]')) return;
    e.preventDefault();
  });
  // all'avvio si parte sempre dalla pagina Stazione (scelta del proprietario); fa eccezione solo il collegamento di associazione del telefono
  const solo = location.hash.startsWith('#/finestra/');
  if (!solo && !location.hash.startsWith('#/associa') && !location.hash.startsWith('#/pair')) history.replaceState(null, '', '#/');
  // i moduli che leggono le preferenze all'avvio si caricano solo ora
  const { App } = await import('./App');
  const { Splash } = await import('./components/Splash');
  if (root) {
    createRoot(root).render(
      <StrictMode>
        <QueryClientProvider client={queryClient}>
          <App />
          {solo ? null : <Splash />}
        </QueryClientProvider>
      </StrictMode>,
    );
  }
}
void start();

// aggiornamento di Aurora sul PC: il telefono si ricarica da solo con la pagina nuova, senza cache
if (import.meta.env.PROD) watchFreshUi();

if ('serviceWorker' in navigator && import.meta.env.PROD) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js').catch(() => {
      // senza service worker l'app funziona lo stesso, solo non offline
    });
  });
}
