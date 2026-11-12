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
import './styles/appearance.css';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { watchFreshUi } from './lib/freshUi';
import { readScale, startUiScale, useUiScale } from './lib/uiScale';
import { applyAppearance, readAppearance, startAppearance } from './lib/appearance';
import { applyTheme, browserStorage, readThemeChoice, resolveTheme } from './lib/theme';
import { deviceKind, hydrate, keepSynced } from './lib/syncSettings';
import { readFilters, useFilters } from './lib/filters';
import { putBandFilters } from './api/client';

// stile dei pulsanti e dei riquadri di tutta l'app (styles/bozze.css)
document.documentElement.dataset.bozza = 'a';

// tema e aspetto (carattere, colore del testo, alto contrasto) subito, prima del primo disegno
applyTheme(document.documentElement, resolveTheme(readThemeChoice(browserStorage())));
startAppearance();

const queryClient = new QueryClient();

// l'ultima casella di testo su cui si e' aperto il menu del tasto destro
let lastField: HTMLInputElement | HTMLTextAreaElement | null = null;

// «Cancella tutto»: svuota la casella con il setter nativo di `value` e poi lancia `input`, cosi' i componenti controllati di React
// (che scavalcano l'assegnazione diretta) vedono il cambio e aggiornano il loro stato, come se il testo fosse stato cancellato a mano
function clearLastField() {
  const field = lastField;
  if (!field || !field.isConnected || field.readOnly || field.disabled || field.value === '') return;
  const proto = field instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
  const setter = Object.getOwnPropertyDescriptor(proto, 'value')?.set;
  if (setter) setter.call(field, '');
  else field.value = '';
  field.dispatchEvent(new Event('input', { bubbles: true }));
  field.focus();
}
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
  // primo avvio senza lingua scelta: quella del sistema o del browser (olandese compreso), se Aurora la conosce. La lingua si carica solo
  // ora, dopo le preferenze portate dal server: caricata prima, partirebbe dall'italiano e ignorerebbe quella salvata.
  try {
    const { firstLaunchLang, useLang } = await import('./i18n/lang');
    const detected = firstLaunchLang(window.localStorage, navigator.languages);
    if (detected && kind === 'desktop') useLang.getState().setLang(detected);
    // sul telefono la lingua e' una scelta condivisa con il PC: quella rilevata vale solo qui e non si salva (finirebbe nelle impostazioni del PC)
    else if (detected) {
      document.documentElement.lang = detected;
      useLang.setState({ lang: detected });
    }
  } catch {
    // senza archivio del browser resta la lingua predefinita
  }
  // i filtri di Attivita' di banda portati dal server: l'elenco li usa subito e le Automazioni li ricevono (non rispondono a chi nascondono)
  try {
    const filters = readFilters(window.localStorage);
    useFilters.setState({ filters });
    void putBandFilters(filters);
  } catch {
    // senza archivio del browser restano i filtri predefiniti
  }
  // la dimensione del testo: dopo le preferenze portate dal server
  // le preferenze portate dal server possono cambiare l'aspetto scelto
  applyTheme(document.documentElement, resolveTheme(readThemeChoice(browserStorage())));
  applyAppearance(document.documentElement, readAppearance());
  useUiScale.setState({ scale: readScale() });
  startUiScale();
  // il menu del tasto destro del browser (Indietro, Ricarica, Ispeziona...) non serve: resta solo nei campi di testo, per copia e incolla
  document.addEventListener('contextmenu', (e) => {
    const target = e.target instanceof Element ? e.target : null;
    // la casella su cui si e' cliccato: la svuota la voce «Cancella tutto» del menu (programma Windows, desktop/src-tauri/src/context_menu.rs)
    const field = target?.closest('input, textarea');
    lastField = field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement ? field : null;
    if (target?.closest('input, textarea, select, [contenteditable="true"]')) return;
    e.preventDefault();
  });
  (window as Window & { __auroraClearField?: () => void }).__auroraClearField = clearLastField;
  // all'avvio si parte sempre dalla pagina Stazione (scelta del proprietario); fanno eccezione le finestre staccate,
  // il collegamento di associazione del telefono e la pagina dei messaggi dell'autore (aperta dalla sua icona sul desktop)
  const solo = location.hash.startsWith('#/finestra/');
  // ricaricando la pagina (F5, «Ricarica») si resta dove si era: solo l'apertura di Aurora riparte dalla Stazione
  const reload = (performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming | undefined)?.type === 'reload';
  const keep = reload || ['#/finestra/', '#/associa', '#/pair', '#/messaggi-autore'].some((h) => location.hash.startsWith(h));
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

// la finestra del programma (Tauri) legge sempre la pagina dal core accanto: niente service worker, e quello di una versione vecchia si toglie,
// cosi' dopo un aggiornamento non resta mai la grafica di prima
const inDesktop = '__TAURI_INTERNALS__' in window;
if (inDesktop && 'serviceWorker' in navigator) {
  void navigator.serviceWorker.getRegistrations().then((regs) => regs.forEach((r) => void r.unregister())).catch(() => undefined);
}
if ('serviceWorker' in navigator && import.meta.env.PROD && !inDesktop) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js').catch(() => {
      // senza service worker l'app funziona lo stesso, solo non offline
    });
  });
}
