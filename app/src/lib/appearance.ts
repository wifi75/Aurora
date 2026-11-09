import { browserStorage } from './theme';

/**
 * Aspetto dell'interfaccia (Impostazioni > Aspetto): alto contrasto, carattere dell'interfaccia e dei dati, colore del testo.
 * Tutto si applica con attributi e variabili CSS sulla radice (`html`), quindi vale per ogni pagina e ogni finestra.
 * Si salva nella chiave `aurora-appearance` delle preferenze locali (copiata sul server come gli altri gusti grafici).
 * La dimensione del testo resta in `lib/uiScale.ts` (chiave `aurora-ui-scale`).
 */
const KEY = 'aurora-appearance';
/** Evento della stessa finestra: le altre finestre lo sanno dall'evento `storage`. */
export const APPEARANCE_EVENT = 'aurora-appearance-change';

export interface FontDef {
  id: string;
  label: string;
  stack: string;
}

/** Caratteri dell'interfaccia: solo font gia' inclusi (IBM Plex Sans) o di sistema, con ripieghi per Windows, macOS e Linux. */
export const UI_FONTS: readonly FontDef[] = [
  { id: 'plex', label: 'IBM Plex Sans', stack: "'IBM Plex Sans', system-ui, sans-serif" },
  { id: 'system', label: 'System UI', stack: "system-ui, -apple-system, 'Segoe UI', Ubuntu, Cantarell, sans-serif" },
  { id: 'segoe', label: 'Segoe UI', stack: "'Segoe UI', 'Helvetica Neue', 'Noto Sans', 'DejaVu Sans', sans-serif" },
  { id: 'verdana', label: 'Verdana', stack: "Verdana, 'DejaVu Sans', 'Bitstream Vera Sans', Geneva, sans-serif" },
  { id: 'tahoma', label: 'Tahoma', stack: "Tahoma, Verdana, 'DejaVu Sans', Geneva, sans-serif" },
  { id: 'arial', label: 'Arial', stack: "Arial, 'Helvetica Neue', Helvetica, 'Liberation Sans', sans-serif" },
  { id: 'calibri', label: 'Calibri', stack: "Calibri, Carlito, 'Segoe UI', 'Helvetica Neue', sans-serif" },
];

/** Caratteri dei dati e dei numeri: monospazio, JetBrains Mono e' gia' incluso nel programma. */
export const DATA_FONTS: readonly FontDef[] = [
  { id: 'jetbrains', label: 'JetBrains Mono', stack: "'JetBrains Mono', ui-monospace, Consolas, monospace" },
  { id: 'consolas', label: 'Consolas', stack: "Consolas, Menlo, 'DejaVu Sans Mono', ui-monospace, monospace" },
  { id: 'cascadia', label: 'Cascadia Mono', stack: "'Cascadia Mono', 'Cascadia Code', Consolas, Menlo, ui-monospace, monospace" },
  { id: 'courier', label: 'Courier New', stack: "'Courier New', Courier, 'Liberation Mono', monospace" },
];

/** Tavolozze di testo leggibile per tema (sul fondo del tema il contrasto resta alto). */
export const TEXT_PALETTE = {
  dark: ['#ffffff', '#e6edf8', '#f4efe2', '#dff6ff', '#fff4b8', '#d9ffe0'],
  light: ['#000000', '#0b1322', '#1c1c1c', '#0f2a4a', '#2e1f0c', '#0d3320'],
} as const;

export interface Appearance {
  /** Alto contrasto: fondo neutro, testo pieno, bordi netti, colori di stato saturi (in chiaro e in scuro). */
  contrast: 'normal' | 'high';
  fontUi: string;
  fontData: string;
  /** Colore del testo per il tema scuro e per quello chiaro; vuoto = quello del tema. */
  textDark: string;
  textLight: string;
}

export const DEFAULT_APPEARANCE: Appearance = { contrast: 'normal', fontUi: 'plex', fontData: 'jetbrains', textDark: '', textLight: '' };

const HEX = /^#[0-9a-f]{6}$/i;

function clean(raw: unknown): Appearance {
  const o = (raw && typeof raw === 'object' ? raw : {}) as Partial<Record<keyof Appearance, unknown>>;
  const pickId = (v: unknown, list: readonly FontDef[], d: string) => (typeof v === 'string' && list.some((f) => f.id === v) ? v : d);
  const color = (v: unknown) => (typeof v === 'string' && HEX.test(v) ? v.toLowerCase() : '');
  return {
    contrast: o.contrast === 'high' ? 'high' : 'normal',
    fontUi: pickId(o.fontUi, UI_FONTS, DEFAULT_APPEARANCE.fontUi),
    fontData: pickId(o.fontData, DATA_FONTS, DEFAULT_APPEARANCE.fontData),
    textDark: color(o.textDark),
    textLight: color(o.textLight),
  };
}

export function readAppearance(): Appearance {
  try {
    const v = browserStorage()?.getItem(KEY);
    return v ? clean(JSON.parse(v)) : { ...DEFAULT_APPEARANCE };
  } catch {
    return { ...DEFAULT_APPEARANCE };
  }
}

export function saveAppearance(a: Appearance): void {
  const v = clean(a);
  try {
    browserStorage()?.setItem(KEY, JSON.stringify(v));
  } catch {
    // senza archivio la scelta vale finche' resta aperta la finestra
  }
  applyAppearance(document.documentElement, v);
  window.dispatchEvent(new CustomEvent(APPEARANCE_EVENT));
}

/** Mette la scelta sulla radice: attributi per i blocchi di `styles/appearance.css`, variabili per caratteri e colori. */
export function applyAppearance(root: HTMLElement, a: Appearance): void {
  const ui = UI_FONTS.find((f) => f.id === a.fontUi);
  const data = DATA_FONTS.find((f) => f.id === a.fontData);
  const set = (name: string, value: string | undefined, isDefault: boolean) => {
    if (!value || isDefault) root.style.removeProperty(name);
    else root.style.setProperty(name, value);
  };
  set('--font-ui', ui?.stack, a.fontUi === DEFAULT_APPEARANCE.fontUi);
  set('--font-data', data?.stack, a.fontData === DEFAULT_APPEARANCE.fontData);
  if (a.contrast === 'high') root.dataset.contrast = 'high';
  else delete root.dataset.contrast;
  // il colore scelto passa per variabili proprie: il CSS lo usa solo nel tema giusto (e cambia da solo col tema)
  set('--user-text-dark', a.textDark, !a.textDark);
  set('--user-text-light', a.textLight, !a.textLight);
  if (a.textDark) root.dataset.textDark = '1';
  else delete root.dataset.textDark;
  if (a.textLight) root.dataset.textLight = '1';
  else delete root.dataset.textLight;
}

/** All'avvio, prima del primo disegno; poi segue le altre finestre (finestre staccate, Impostazioni). */
export function startAppearance(): void {
  applyAppearance(document.documentElement, readAppearance());
  window.addEventListener('storage', (e) => {
    if (e.key === KEY) applyAppearance(document.documentElement, readAppearance());
  });
}

/* Contrasto WCAG tra due colori esadecimali (#rrggbb o #rgb); null se uno dei due non si legge. */
function lum(hex: string): number | null {
  let h = hex.trim().replace('#', '');
  if (h.length === 3) h = h.split('').map((c) => c + c).join('');
  if (!/^[0-9a-f]{6}$/i.test(h)) return null;
  const ch = [0, 2, 4].map((i) => {
    const c = parseInt(h.slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * ch[0]! + 0.7152 * ch[1]! + 0.0722 * ch[2]!;
}

export function contrastRatio(a: string, b: string): number | null {
  const la = lum(a);
  const lb = lum(b);
  if (la === null || lb === null) return null;
  const [hi, lo] = la > lb ? [la, lb] : [lb, la];
  return (hi + 0.05) / (lo + 0.05);
}
