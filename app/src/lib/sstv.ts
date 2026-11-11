/**
 * SSTV: logica pura della pagina (niente DOM), provata a parte. Composizione per livelli in coordinate relative (0-1) all'immagine,
 * cosi' cambiare modo (320x256, 640x496...) non sposta niente; ritaglio della foto di sfondo; segnaposto dei modelli; ritocchi di
 * luminosita', contrasto e saturazione; annulla/ripeti; operazioni sui livelli; modelli salvati in JSON.
 */

export type Fill = { kind: 'solid'; color: string } | { kind: 'gradient'; from: string; to: string; angle: number };

export interface TextLayer {
  id: string;
  kind: 'text';
  text: string;
  /** Centro del testo, 0-1 dell'immagine. */
  x: number;
  y: number;
  font: string;
  /** Altezza dei caratteri come frazione dell'altezza dell'immagine. */
  size: number;
  bold: boolean;
  italic: boolean;
  fill: Fill;
  outlineColor: string;
  /** Spessore del contorno in frazione della dimensione dei caratteri (0 = niente). */
  outline: number;
  shadow: boolean;
  shadowColor: string;
  rotation: number;
  align: 'left' | 'center' | 'right';
  box: boolean;
  boxColor: string;
  boxOpacity: number;
  hidden: boolean;
}

export interface ShapeLayer {
  id: string;
  kind: 'shape';
  shape: 'rect' | 'ellipse' | 'frame' | 'bars';
  x: number;
  y: number;
  w: number;
  h: number;
  rotation: number;
  fill: Fill;
  /** Riempito o solo bordo (le cornici sono sempre bordo). */
  filled: boolean;
  strokeColor: string;
  /** Spessore del bordo in frazione dell'altezza dell'immagine. */
  stroke: number;
  opacity: number;
  hidden: boolean;
}

export type Layer = TextLayer | ShapeLayer;

export interface Adjust {
  brightness: number;
  contrast: number;
  saturation: number;
}

export interface Background {
  color: string;
  fit: 'cover' | 'contain';
  zoom: number;
  panX: number;
  panY: number;
  adjust: Adjust;
}

export interface Composition {
  background: Background;
  layers: Layer[];
}

export const FONTS: readonly { id: string; label: string }[] = [
  { id: 'Inter, system-ui, sans-serif', label: 'Sans' },
  { id: 'Georgia, serif', label: 'Serif' },
  { id: 'Anton', label: 'Anton' },
  { id: 'Bebas Neue', label: 'Bebas Neue' },
  { id: 'Bangers', label: 'Bangers' },
  { id: 'Black Ops One', label: 'Black Ops One' },
  { id: 'Orbitron', label: 'Orbitron' },
  { id: 'Righteous', label: 'Righteous' },
  { id: 'Press Start 2P', label: 'Press Start 2P' },
  { id: 'Lobster', label: 'Lobster' },
  { id: 'Pacifico', label: 'Pacifico' },
  { id: 'Great Vibes', label: 'Great Vibes' },
];

/** Tavolozze pronte: colori che si leggono bene anche dopo l'SSTV (saturi, contrastati). */
export const PALETTES: readonly (readonly string[])[] = [
  ['#ffffff', '#000000', '#ffd400', '#ff3b30', '#34c759', '#0a84ff', '#ff9f0a', '#bf5af2'],
  ['#fff4e0', '#ffb703', '#fb8500', '#e63946', '#8338ec', '#3a86ff', '#06d6a0', '#023047'],
  ['#e0fbfc', '#98c1d9', '#3d5a80', '#293241', '#ee6c4d', '#f4d35e', '#90be6d', '#f94144'],
];

export const NEUTRAL_ADJUST: Adjust = { brightness: 0, contrast: 0, saturation: 0 };

export function emptyComposition(): Composition {
  return { background: { color: '#10243e', fit: 'cover', zoom: 1, panX: 0, panY: 0, adjust: { ...NEUTRAL_ADJUST } }, layers: [] };
}

let counter = 0;
export function newId(): string {
  counter += 1;
  return `l${Date.now().toString(36)}${counter}`;
}

export function textLayer(text: string, over: Partial<TextLayer> = {}): TextLayer {
  return {
    id: newId(),
    kind: 'text',
    text,
    x: 0.5,
    y: 0.5,
    font: 'Anton',
    size: 0.14,
    bold: false,
    italic: false,
    fill: { kind: 'solid', color: '#ffffff' },
    outlineColor: '#000000',
    outline: 0.08,
    shadow: true,
    shadowColor: '#000000',
    rotation: 0,
    align: 'center',
    box: false,
    boxColor: '#000000',
    boxOpacity: 0.5,
    hidden: false,
    ...over,
  };
}

export function shapeLayer(shape: ShapeLayer['shape'], over: Partial<ShapeLayer> = {}): ShapeLayer {
  const frame = shape === 'frame';
  return {
    id: newId(),
    kind: 'shape',
    shape,
    x: 0.5,
    y: 0.5,
    w: frame ? 1 : 0.4,
    h: frame ? 1 : 0.25,
    rotation: 0,
    fill: { kind: 'solid', color: '#ffd400' },
    filled: !frame,
    strokeColor: '#ffffff',
    stroke: frame ? 0.03 : 0.01,
    opacity: 1,
    hidden: false,
    ...over,
  };
}

/** Durata leggibile: «1:54», «36 s». */
export function formatDuration(seconds: number): string {
  const s = Math.round(seconds);
  if (s < 60) return `${s} s`;
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/**
 * Dove disegnare una foto `srcW`x`srcH` in un'immagine `dstW`x`dstH`: «cover» la riempie tutta (taglia cio' che avanza), «contain» la
 * mostra intera (bande ai lati). `zoom` >= 1 ingrandisce, `panX`/`panY` (-1..1) spostano la parte visibile fin dove la foto arriva.
 */
export function placeImage(srcW: number, srcH: number, dstW: number, dstH: number, fit: 'cover' | 'contain', zoom = 1, panX = 0, panY = 0): Rect {
  if (srcW <= 0 || srcH <= 0) return { x: 0, y: 0, w: dstW, h: dstH };
  const base = fit === 'cover' ? Math.max(dstW / srcW, dstH / srcH) : Math.min(dstW / srcW, dstH / srcH);
  const scale = base * Math.max(1, zoom);
  const w = srcW * scale;
  const h = srcH * scale;
  const clamp = (v: number) => Math.max(-1, Math.min(1, v));
  // margine di spostamento: quanto la foto sporge (o manca) rispetto all'immagine
  const spareX = (w - dstW) / 2;
  const spareY = (h - dstH) / 2;
  return { x: (dstW - w) / 2 - (spareX > 0 ? clamp(panX) * spareX : 0), y: (dstH - h) / 2 - (spareY > 0 ? clamp(panY) * spareY : 0), w, h };
}

export interface TokenContext {
  mycall?: string | null;
  call?: string | null;
  rst?: string | null;
  grid?: string | null;
  name?: string | null;
  mode?: string | null;
  now?: Date;
}

/** Segnaposto dei modelli: {MYCALL} {CALL} {RST} {GRID} {NAME} {MODE} {DATE} {TIME} (UTC); maiuscole o minuscole. */
export function expandTokens(text: string, ctx: TokenContext): string {
  const now = ctx.now ?? new Date();
  const pad = (n: number) => String(n).padStart(2, '0');
  const values: Record<string, string> = {
    MYCALL: (ctx.mycall ?? '').toUpperCase(),
    CALL: (ctx.call ?? '').toUpperCase(),
    RST: ctx.rst ?? '595',
    GRID: ctx.grid ?? '',
    NAME: ctx.name ?? '',
    MODE: ctx.mode ?? '',
    DATE: `${now.getUTCFullYear()}-${pad(now.getUTCMonth() + 1)}-${pad(now.getUTCDate())}`,
    TIME: `${pad(now.getUTCHours())}:${pad(now.getUTCMinutes())} UTC`,
  };
  return text.replace(/\{([a-zA-Z]+)\}/g, (all, key: string) => values[key.toUpperCase()] ?? all);
}

/** Righe RGB (3 byte per pixel) in RGBA dentro `out` (ImageData.data) a partire dalla riga `firstRow`. */
export function rgbIntoRgba(rgb: Uint8Array, out: Uint8ClampedArray, width: number, firstRow: number): void {
  let o = firstRow * width * 4;
  for (let i = 0; i + 2 < rgb.length && o + 3 < out.length; i += 3, o += 4) {
    out[o] = rgb[i] ?? 0;
    out[o + 1] = rgb[i + 1] ?? 0;
    out[o + 2] = rgb[i + 2] ?? 0;
    out[o + 3] = 255;
  }
}

export function base64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

/** Luminosita', contrasto e saturazione (-100..100) sui pixel RGBA, sul posto. */
export function applyAdjust(data: Uint8ClampedArray, adjust: Adjust): void {
  const { brightness, contrast, saturation } = adjust;
  if (brightness === 0 && contrast === 0 && saturation === 0) return;
  const b = (brightness / 100) * 128;
  const c = Math.max(0, 1 + contrast / 100) ** 2;
  const s = Math.max(0, 1 + saturation / 100);
  for (let i = 0; i < data.length; i += 4) {
    let r = data[i] ?? 0;
    let g = data[i + 1] ?? 0;
    let bl = data[i + 2] ?? 0;
    const y = 0.299 * r + 0.587 * g + 0.114 * bl;
    r = y + (r - y) * s;
    g = y + (g - y) * s;
    bl = y + (bl - y) * s;
    data[i] = (r - 128) * c + 128 + b;
    data[i + 1] = (g - 128) * c + 128 + b;
    data[i + 2] = (bl - 128) * c + 128 + b;
  }
}

/** Annulla/ripeti: istantanee della composizione, al massimo `limit`. */
export interface History<S> {
  past: S[];
  present: S;
  future: S[];
}

export function historyStart<S>(present: S): History<S> {
  return { past: [], present, future: [] };
}

export function historyPush<S>(h: History<S>, next: S, limit = 60): History<S> {
  if (next === h.present) return h;
  return { past: [...h.past, h.present].slice(-limit), present: next, future: [] };
}

export function historyUndo<S>(h: History<S>): History<S> {
  const prev = h.past[h.past.length - 1];
  if (prev === undefined) return h;
  return { past: h.past.slice(0, -1), present: prev, future: [h.present, ...h.future] };
}

export function historyRedo<S>(h: History<S>): History<S> {
  const [next, ...rest] = h.future;
  if (next === undefined) return h;
  return { past: [...h.past, h.present], present: next, future: rest };
}

/** Sposta un livello: `+1` sopra (verso la fine dell'elenco, disegnato dopo), `-1` sotto. */
export function moveLayer(layers: readonly Layer[], id: string, dir: 1 | -1): Layer[] {
  const i = layers.findIndex((l) => l.id === id);
  const j = i + dir;
  if (i < 0 || j < 0 || j >= layers.length) return [...layers];
  const out = [...layers];
  const a = out[i];
  const b = out[j];
  if (!a || !b) return out;
  out[i] = b;
  out[j] = a;
  return out;
}

export function duplicateLayer(layers: readonly Layer[], id: string): Layer[] {
  const i = layers.findIndex((l) => l.id === id);
  const src = layers[i];
  if (!src) return [...layers];
  const copy = { ...src, id: newId(), x: Math.min(0.95, src.x + 0.03), y: Math.min(0.95, src.y + 0.03) } as Layer;
  return [...layers.slice(0, i + 1), copy, ...layers.slice(i + 1)];
}

export function updateLayer(layers: readonly Layer[], id: string, patch: Partial<TextLayer> | Partial<ShapeLayer>): Layer[] {
  return layers.map((l) => (l.id === id ? ({ ...l, ...patch } as Layer) : l));
}

/** Il punto (px, py) e' dentro il rettangolo di centro (cx, cy), lati w x h, ruotato di `deg` gradi? */
export function pointInRotatedRect(px: number, py: number, cx: number, cy: number, w: number, h: number, deg: number): boolean {
  const a = (-deg * Math.PI) / 180;
  const dx = px - cx;
  const dy = py - cy;
  const lx = dx * Math.cos(a) - dy * Math.sin(a);
  const ly = dx * Math.sin(a) + dy * Math.cos(a);
  return Math.abs(lx) <= w / 2 && Math.abs(ly) <= h / 2;
}

/** Angolo (gradi, 0 = in alto, senso orario) del punto rispetto al centro: per la maniglia di rotazione. */
export function angleFrom(cx: number, cy: number, px: number, py: number): number {
  const deg = (Math.atan2(px - cx, -(py - cy)) * 180) / Math.PI;
  return Math.round(((deg % 360) + 360) % 360);
}

/** Modelli pronti, con i segnaposto. */
export function presetLayers(kind: 'cq' | 'reply' | 'qsl'): Layer[] {
  if (kind === 'cq') {
    return [
      textLayer('CQ SSTV', { y: 0.2, size: 0.2, fill: { kind: 'gradient', from: '#ffd400', to: '#ff3b30', angle: 90 } }),
      textLayer('DE {MYCALL}', { y: 0.72, size: 0.15, font: 'Bebas Neue' }),
      textLayer('{GRID}', { y: 0.88, size: 0.08, font: 'Orbitron', box: true }),
    ];
  }
  if (kind === 'reply') {
    return [
      textLayer('{CALL} DE {MYCALL}', { y: 0.16, size: 0.13, font: 'Bebas Neue', box: true, boxOpacity: 0.55 }),
      textLayer('UR {RST}', { y: 0.82, size: 0.16, fill: { kind: 'solid', color: '#ffd400' } }),
    ];
  }
  return [
    textLayer('TNX QSO {CALL}', { y: 0.18, size: 0.13, font: 'Righteous' }),
    textLayer('73', { y: 0.55, size: 0.32, font: 'Lobster', fill: { kind: 'gradient', from: '#ffffff', to: '#0a84ff', angle: 90 } }),
    textLayer('{MYCALL}  {DATE}', { y: 0.88, size: 0.08, font: 'Orbitron', box: true }),
  ];
}

/** Modello salvato (nel browser): la composizione e, se non troppo grande, la foto di sfondo come data URL. */
export interface SavedTemplate {
  name: string;
  composition: Composition;
  background?: string;
}

function isObj(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null;
}

/** Legge i modelli salvati, scartando quel che non ha la forma giusta. */
export function parseTemplates(json: string | null): SavedTemplate[] {
  if (!json) return [];
  try {
    const data: unknown = JSON.parse(json);
    if (!Array.isArray(data)) return [];
    return data.filter((t): t is SavedTemplate => isObj(t) && typeof t.name === 'string' && isObj(t.composition) && Array.isArray(t.composition.layers) && isObj(t.composition.background));
  } catch {
    return [];
  }
}
