/**
 * L'ambiente dell'interfaccia, per il pacchetto di assistenza.
 *
 * Il programma non vede cosa c'e' sullo schermo: dimensione e zoom della finestra, tema, lingua, la pagina aperta, gli errori della pagina
 * (quelli che il registro del programma non ha). La pagina li manda al programma (`POST /api/v1/diagnostics/ui-snapshot`) all'avvio, quando
 * la finestra cambia e prima di mandare il pacchetto; il programma li mette in `diagnostica/interfaccia-*.json`.
 * Niente dati personali: dei valori salvati nel browser si dicono solo i nomi e la dimensione, mai il contenuto.
 */

declare const __AURORA_UI_VERSION__: string;

/** Quanti errori si ricordano. */
const ERRORS_MAX = 30;
/** Una riga d'errore si taglia a tanto. */
const ERROR_CLIP = 400;
/** Il programma non accetta piu' di 16 KB: si resta sotto. */
const SNAPSHOT_MAX = 14_000;

interface PageError {
  /** Ora del browser (ISO, UTC). */
  at: string;
  kind: 'error' | 'promise' | 'console';
  text: string;
}

const errors: PageError[] = [];
let installed = false;

function clip(text: string, max = ERROR_CLIP): string {
  const one = text.replace(/\s+/g, ' ').trim();
  return one.length > max ? `${one.slice(0, max)}…` : one;
}

function remember(kind: PageError['kind'], text: string): void {
  errors.push({ at: new Date().toISOString(), kind, text: clip(text) });
  if (errors.length > ERRORS_MAX) errors.shift();
}

function describe(value: unknown): string {
  if (value instanceof Error) return `${value.name}: ${value.message}${value.stack ? ` @ ${value.stack.split('\n').slice(1, 3).join(' ')}` : ''}`;
  if (typeof value === 'string') return value;
  try {
    return JSON.stringify(value) ?? String(value);
  } catch {
    return String(value);
  }
}

/** Gli errori ricordati (per le prove). */
export function recentErrors(): readonly PageError[] {
  return errors;
}

/** L'ultima pagina aperta che non fosse le Impostazioni (il pacchetto parte spesso da li', ma il guaio e' nella pagina di prima). */
let lastWorkPage = '';

/** Le scelte di layout salvate nel browser di cui si dice anche il valore: sono numeri e nomi di colonne, mai nominativi o locatori. */
const LAYOUT_KEYS = /^(aurora\.digi\.(winw|rxfrac|wfh2|macrow|macrorows|radarw|bandh|logw|ssbcols|tab|viste)(\.|$)|aurora-layout$|aurora-finestra-rx$|aurora\.contest\.layout\.|aurora\.rxw\.cols$)/;

function savedLayout(): Record<string, string> {
  const out: Record<string, string> = {};
  try {
    const store = window.localStorage;
    for (let i = 0; i < store.length && Object.keys(out).length < 60; i++) {
      const key = store.key(i);
      if (key !== null && LAYOUT_KEYS.test(key)) out[key] = (store.getItem(key) ?? '').slice(0, 300);
    }
  } catch {
    // senza archivio del browser
  }
  return out;
}

/** Misure (larghezza e altezza, in px) dei riquadri principali della pagina aperta: dicono se uno e' stretto o schiacciato. */
const PANELS = ['.digi-windows', '.digi-rxwin', '.digi-txwrap', '.digi-split', '.digi-log', '.digi-macros', '.decode-list', '.app-shell', '.ssbc', '.ssbc-cols', '.ctw-panel'];

function panelSizes(): Record<string, [number, number, number]> {
  const out: Record<string, [number, number, number]> = {};
  try {
    for (const sel of PANELS) {
      const els = document.querySelectorAll(sel);
      const first = els.item(0);
      if (!first) continue;
      const r = first.getBoundingClientRect();
      // larghezza, altezza e quanti ce ne sono con quel nome
      out[sel] = [Math.round(r.width), Math.round(r.height), els.length];
    }
  } catch {
    // le misure sono un di piu'
  }
  return out;
}

/** Il nome della pagina aperta, senza i parametri (potrebbero contenere codici). */
function route(): string {
  return (window.location.hash || '#/').split('?')[0] ?? '#/';
}

function noteWorkPage(): void {
  const here = route();
  if (!here.startsWith('#/impostazioni') && here !== '#/') lastWorkPage = here;
  // la pagina di partenza (#/) e' l'FT8: si ricorda quando e' l'ultima in cui si e' stati
  else if (here === '#/') lastWorkPage = here;
}

function storageInfo(): { keys: number; bytes: number; aurora: Record<string, number> } {
  const aurora: Record<string, number> = {};
  let bytes = 0;
  let keys = 0;
  try {
    const store = window.localStorage;
    keys = store.length;
    for (let i = 0; i < store.length; i++) {
      const key = store.key(i);
      if (key === null) continue;
      const size = (store.getItem(key) ?? '').length;
      bytes += key.length + size;
      // solo i nomi e le dimensioni: il contenuto puo' avere nominativi e locatori
      if (key.startsWith('aurora') && Object.keys(aurora).length < 120) aurora[key] = size;
    }
  } catch {
    // senza archivio del browser
  }
  return { keys, bytes, aurora };
}

function gpu(): string | null {
  try {
    const canvas = document.createElement('canvas');
    const gl = (canvas.getContext('webgl2') ?? canvas.getContext('webgl')) as WebGLRenderingContext | null;
    if (!gl) return 'nessuno (niente WebGL)';
    const info = gl.getExtension('WEBGL_debug_renderer_info');
    return info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : 'WebGL senza nome';
  } catch {
    return null;
  }
}

/** Cio' che la pagina sa del proprio ambiente. */
export function collectSnapshot(): Record<string, unknown> {
  noteWorkPage();
  const nav = navigator as Navigator & { deviceMemory?: number };
  const doc = document.documentElement;
  return {
    schema: 1,
    raccolto_il: new Date().toISOString(),
    versione_interfaccia: typeof __AURORA_UI_VERSION__ === 'string' ? __AURORA_UI_VERSION__ : '?',
    programma_di_aurora: '__TAURI_INTERNALS__' in window || '__TAURI__' in window,
    pagina: route(),
    ultima_pagina_di_lavoro: lastWorkPage || route(),
    riquadri_della_pagina: panelSizes(),
    layout_salvato: savedLayout(),
    lingua: doc.lang,
    tema: doc.dataset.theme ?? null,
    finestra: {
      interna: [window.innerWidth, window.innerHeight],
      esterna: [window.outerWidth, window.outerHeight],
      schermo: [window.screen.width, window.screen.height],
      disponibile: [window.screen.availWidth, window.screen.availHeight],
      scala_dello_schermo: window.devicePixelRatio,
      a_schermo_intero: window.matchMedia('(display-mode: fullscreen)').matches || (window.screen.height === window.innerHeight && window.screen.width === window.innerWidth),
      verticale: window.matchMedia('(orientation: portrait)').matches,
    },
    dispositivo: {
      tocco: nav.maxTouchPoints,
      cpu_logiche: nav.hardwareConcurrency,
      memoria_gb: nav.deviceMemory ?? null,
      lingue_del_browser: [...navigator.languages],
      fuso_orario_minuti: new Date().getTimezoneOffset(),
      scheda_grafica: gpu(),
      tema_del_sistema_scuro: window.matchMedia('(prefers-color-scheme: dark)').matches,
      movimento_ridotto: window.matchMedia('(prefers-reduced-motion: reduce)').matches,
    },
    browser: navigator.userAgent,
    archivio_del_browser: storageInfo(),
    errori_della_pagina: errors,
  };
}

/** Il testo da mandare, tagliato a misura (si tolgono prima i nomi dell'archivio, poi gli errori piu' vecchi). */
export function snapshotText(): string {
  const snap = collectSnapshot();
  let text = JSON.stringify(snap);
  if (text.length > SNAPSHOT_MAX) {
    snap.archivio_del_browser = { ...(snap.archivio_del_browser as object), aurora: '(troppi per stare nel pacchetto)' };
    text = JSON.stringify(snap);
  }
  while (text.length > SNAPSHOT_MAX && errors.length > 3) {
    errors.shift();
    snap.errori_della_pagina = errors;
    text = JSON.stringify(snap);
  }
  return text.length > SNAPSHOT_MAX ? text.slice(0, SNAPSHOT_MAX) : text;
}

let lastSent = 0;

/** Manda i dati al programma (non si aspetta oltre `waitMs`; senza programma raggiungibile non fa niente). */
export async function sendUiSnapshot(waitMs = 4000): Promise<void> {
  lastSent = Date.now();
  const abort = new AbortController();
  const timer = window.setTimeout(() => abort.abort(), waitMs);
  try {
    await fetch('/api/v1/diagnostics/ui-snapshot', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: snapshotText(),
      credentials: 'same-origin',
      signal: abort.signal,
    });
  } catch {
    // il programma non risponde o l'accesso non c'e': la diagnostica non deve mai dare fastidio
  } finally {
    window.clearTimeout(timer);
  }
}

/** Si mette in ascolto degli errori della pagina e manda i dati: all'avvio, quando la finestra cambia, ogni 5 minuti. */
export function installUiDiagnostics(): void {
  if (installed) return;
  installed = true;
  window.addEventListener('error', (e) => remember('error', `${e.message || 'errore'} (${e.filename ? e.filename.split('/').pop() : '?'}:${e.lineno}:${e.colno})`));
  window.addEventListener('unhandledrejection', (e) => remember('promise', describe(e.reason)));
  // gli errori che la pagina stessa scrive nella console (anche quelli di React, come «pagina in errore»)
  const original = console.error.bind(console);
  console.error = (...args: unknown[]) => {
    try {
      remember('console', args.map(describe).join(' '));
    } catch {
      // la diagnostica non deve rompere la console
    }
    original(...args);
  };
  let resizeTimer: number | undefined;
  window.addEventListener('resize', () => {
    window.clearTimeout(resizeTimer);
    resizeTimer = window.setTimeout(() => void sendUiSnapshot(), 8000);
  });
  window.addEventListener('hashchange', () => {
    noteWorkPage();
    // al massimo una volta al minuto per i cambi di pagina
    if (Date.now() - lastSent > 60_000) void sendUiSnapshot();
  });
  window.setTimeout(() => void sendUiSnapshot(), 5000);
  window.setInterval(() => void sendUiSnapshot(), 5 * 60_000);
}
