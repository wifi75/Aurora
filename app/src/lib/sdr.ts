import type { AudioValues, CatConfig } from '../api/client';
import { browserStorage } from './theme';

/** I nomi dell'ingresso e dell'uscita audio TCI come li elenca il motore (`TCI_INPUT_NAME` in `audio-spectrum/src/tci_source.rs`, `TCI_OUTPUT_NAME` in `tx-native/src/tci_sink.rs`). */
export const TCI_INPUT = 'TCI (dalla radio)';
export const TCI_OUTPUT = 'TCI (alla radio)';
/** L'indirizzo del server TCI quando il campo e' vuoto (lo stesso del motore). */
export const TCI_DEFAULT_ADDRESS = '127.0.0.1:40001';

const same = (a: string, b: string) => a.trim().toLowerCase() === b.toLowerCase();

/** Vero se CAT, ingresso e uscita audio passano gia' tutti dal TCI. */
export function tciEverywhere(cat: Pick<CatConfig, 'backend'>, audio: Pick<AudioValues, 'input' | 'output'>): boolean {
  return cat.backend === 'tci' && same(audio.input, TCI_INPUT) && same(audio.output, TCI_OUTPUT);
}

/** Chiave dell'archivio con cui una finestra a parte chiede alla finestra principale di aprire una pagina. */
export const NAVIGATE_KEY = 'aurora.navigate';

/**
 * Apre una pagina di Aurora (`#/qmap`, `#/guida/...`) nella finestra principale. Dentro il riquadro delle Impostazioni basta cambiare
 * l'indirizzo; dalla finestra a parte (`#/finestra/...`) la richiesta passa dall'archivio, che la finestra principale ascolta.
 */
export function openInMain(hash: string): void {
  if (!location.hash.startsWith('#/finestra/')) {
    location.hash = hash;
    return;
  }
  try {
    browserStorage()?.setItem(NAVIGATE_KEY, JSON.stringify({ hash, at: Date.now() }));
  } catch {
    // senza archivio non si puo' parlare con la finestra principale: la pagina si apre da li'
  }
}

/** Nella finestra principale: segue le richieste di `openInMain` delle finestre a parte. Restituisce la funzione per smettere. */
export function followNavigateRequests(): () => void {
  const onStorage = (e: StorageEvent) => {
    if (e.key !== NAVIGATE_KEY || !e.newValue) return;
    try {
      const { hash, at } = JSON.parse(e.newValue) as { hash?: unknown; at?: unknown };
      // solo pagine di Aurora e solo richieste fresche
      if (typeof hash === 'string' && /^#\/[a-z0-9/-]{1,40}$/.test(hash) && typeof at === 'number' && Date.now() - at < 10_000) location.hash = hash;
    } catch {
      // richiesta illeggibile: si ignora
    }
  };
  window.addEventListener('storage', onStorage);
  return () => window.removeEventListener('storage', onStorage);
}
