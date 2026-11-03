/** La versione dell'interfaccia scritta al momento della compilazione (vite.config.ts). */
declare const __AURORA_UI_VERSION__: string;

const MARK = 'aurora-fresh-ui';

/**
 * Telefono e tablet: quando Aurora sul PC viene aggiornata, la pagina si ricarica da sola senza cache.
 * Si confronta la versione del core (`/api/v1/health`, mai dalla cache) con quella di questa interfaccia: se sono diverse si
 * tolgono service worker e cache e si ricarica. Una sola prova per versione, cosi' non si entra mai in un giro di ricariche.
 */
export async function checkFreshUi(): Promise<void> {
  const mine = typeof __AURORA_UI_VERSION__ === 'string' ? __AURORA_UI_VERSION__ : '';
  if (mine === '') return;
  let server = '';
  try {
    const r = await fetch('/api/v1/health', { cache: 'no-store' });
    if (!r.ok) return;
    server = ((await r.json()) as { version?: string }).version ?? '';
  } catch {
    return;
  }
  if (server === '' || server === mine) return;
  try {
    if (sessionStorage.getItem(MARK) === server) return;
    sessionStorage.setItem(MARK, server);
  } catch {
    // senza sessionStorage si prova lo stesso una volta
  }
  try {
    const regs = (await navigator.serviceWorker?.getRegistrations?.()) ?? [];
    await Promise.all(regs.map((r) => r.unregister()));
    const keys = (await caches?.keys?.()) ?? [];
    await Promise.all(keys.map((k) => caches.delete(k)));
  } catch {
    // anche senza pulizia la ricarica forzata prende la pagina nuova
  }
  const url = new URL(window.location.href);
  url.searchParams.set('v', server);
  window.location.replace(url.toString());
}

/** Controllo all'avvio, quando la pagina torna in primo piano e ogni minuto. */
export function watchFreshUi(): void {
  void checkFreshUi();
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') void checkFreshUi();
  });
  setInterval(() => void checkFreshUi(), 60_000);
}
