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
import { readScale, startUiScale, useUiScale } from './lib/uiScale';
import { deviceKind, hydrate, keepSynced } from './lib/syncSettings';

// stile dei pulsanti e dei riquadri di tutta l'app (styles/bozze.css)
document.documentElement.dataset.bozza = 'a';

const queryClient = new QueryClient();
const root = document.getElementById('root');

// le preferenze si portano dal server prima di disegnare (i moduli le leggono all'avvio) e poi si tengono allineate
async function start() {
  const kind = deviceKind(window.innerWidth);
  try {
    await hydrate(window.localStorage, kind);
    // prova locale nel browser: prende la configurazione del PC ma non la riscrive (le prove non toccano l'Aurora installata)
    if (import.meta.env.VITE_AURORA_PROVA !== '1') keepSynced(kind);
  } catch {
    // senza archivio del browser l'interfaccia parte con i valori predefiniti
  }
  // la dimensione del testo: dopo le preferenze portate dal server
  useUiScale.setState({ scale: readScale() });
  startUiScale();
  // il menu del tasto destro del browser (Indietro, Ricarica, Ispeziona...) non serve: resta solo nei campi di testo, per copia e incolla
  document.addEventListener('contextmenu', (e) => {
    const target = e.target instanceof Element ? e.target : null;
    if (target?.closest('input, textarea, select, [contenteditable="true"]')) return;
    e.preventDefault();
  });
  // all'avvio si parte sempre dalla pagina Stazione (scelta del proprietario); fanno eccezione le finestre staccate,
  // il collegamento di associazione del telefono e la pagina dei messaggi dell'autore (aperta dalla sua icona sul desktop)
  const solo = location.hash.startsWith('#/finestra/');
  const keep = ['#/finestra/', '#/associa', '#/pair', '#/messaggi-autore'].some((h) => location.hash.startsWith(h));
  if (!keep) history.replaceState(null, '', '#/');
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
