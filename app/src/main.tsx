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

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { deviceKind, hydrate, keepSynced } from './lib/syncSettings';

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
  // senza un indirizzo preciso si riparte dall'ultima pagina aperta
  const { lastPageToRestore } = await import('./lib/lastPage');
  const { browserStorage } = await import('./lib/theme');
  const restore = lastPageToRestore(browserStorage(), location.hash);
  if (restore) history.replaceState(null, '', restore);
  // i moduli che leggono le preferenze all'avvio si caricano solo ora
  const { App } = await import('./App');
  const { Splash } = await import('./components/Splash');
  if (root) {
    createRoot(root).render(
      <StrictMode>
        <QueryClientProvider client={queryClient}>
          <App />
          <Splash />
        </QueryClientProvider>
      </StrictMode>,
    );
  }
}
void start();

if ('serviceWorker' in navigator && import.meta.env.PROD) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js').catch(() => {
      // senza service worker l'app funziona lo stesso, solo non offline
    });
  });
}
