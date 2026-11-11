import { applyAdjust, expandTokens, placeImage, type Composition, type Fill, type Layer, type ShapeLayer, type TextLayer, type TokenContext } from './sstv';

/** Sorgente disegnabile della foto di sfondo (ImageBitmap o canvas gia' ridotto). */
export type BgSource = ImageBitmap | HTMLCanvasElement | HTMLImageElement;

function sourceSize(src: BgSource): { w: number; h: number } {
  if (src instanceof HTMLImageElement) return { w: src.naturalWidth, h: src.naturalHeight };
  return { w: src.width, h: src.height };
}

function paint(ctx: CanvasRenderingContext2D, fill: Fill, x: number, y: number, w: number, h: number): string | CanvasGradient {
  if (fill.kind === 'solid') return fill.color;
  const a = (fill.angle * Math.PI) / 180;
  const cx = x + w / 2;
  const cy = y + h / 2;
  const r = (Math.abs(w * Math.sin(a)) + Math.abs(h * Math.cos(a))) / 2;
  const dx = Math.sin(a) * r;
  const dy = -Math.cos(a) * r;
  // angolo 0 = dal basso verso l'alto, 90 = da sinistra a destra
  const g = ctx.createLinearGradient(cx - dx, cy - dy, cx + dx, cy + dy);
  g.addColorStop(0, fill.from);
  g.addColorStop(1, fill.to);
  return g;
}

function hexAlpha(hex: string, alpha: number): string {
  const m = /^#([0-9a-f]{6})$/i.exec(hex);
  if (!m) return hex;
  const n = parseInt(m[1] ?? '0', 16);
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${Math.max(0, Math.min(1, alpha))})`;
}

export function fontString(l: TextLayer, px: number): string {
  const family = /[,]/.test(l.font) ? l.font : `"${l.font}", system-ui, sans-serif`;
  return `${l.italic ? 'italic ' : ''}${l.bold ? '700 ' : '400 '}${Math.max(1, Math.round(px))}px ${family}`;
}

/** Interlinea delle righe di un testo, in frazioni della dimensione dei caratteri. */
const LINE_HEIGHT = 1.18;

/** Rettangolo in coordinate locali del livello (origine nel suo punto x,y, prima della rotazione). */
export interface Rect {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

/**
 * Misure di una riga come le da' il canvas con lo stesso carattere, il `textAlign` del livello e `textBaseline = 'alphabetic'`:
 * avanzamento e inchiostro vero dei glifi (a sinistra e a destra del punto di ancoraggio, sopra e sotto la linea di base).
 */
export interface LineMetrics {
  width: number;
  left: number;
  right: number;
  ascent: number;
  descent: number;
  /** Riga vuota o di soli spazi: non ha inchiostro. */
  blank: boolean;
}

/** Geometria di un testo: dove disegnare ogni riga perche' l'inchiostro risulti centrato sul punto del livello. */
export interface TextGeometry {
  lines: string[];
  px: number;
  /** Punto di ancoraggio (x) e linea di base (y) di ogni riga, gia' spostati. */
  anchors: { x: number; y: number }[];
  /** Ingombro dei glifi (senza contorno ne' ombra), centrato sull'origine. */
  ink: Rect;
}

function finite(v: number | undefined, fallback: number): number {
  return typeof v === 'number' && Number.isFinite(v) ? v : fallback;
}

/** Carattere, allineamento e linea di base con cui il testo si misura e si disegna: sempre gli stessi nei due casi. */
function setTextStyle(ctx: CanvasRenderingContext2D, l: TextLayer, px: number): void {
  ctx.font = fontString(l, px);
  ctx.textAlign = l.align;
  ctx.textBaseline = 'alphabetic';
}

/** Misura le righe con il canvas; se il browser non da' l'inchiostro (molto vecchio) lo si stima dall'avanzamento. */
export function measureLines(ctx: CanvasRenderingContext2D, l: TextLayer, lines: readonly string[], px: number): LineMetrics[] {
  setTextStyle(ctx, l, px);
  return lines.map((s) => {
    const m = ctx.measureText(s);
    const width = finite(m.width, 0);
    const leftGuess = l.align === 'left' ? 0 : l.align === 'right' ? width : width / 2;
    return {
      width,
      left: finite(m.actualBoundingBoxLeft, leftGuess),
      right: finite(m.actualBoundingBoxRight, width - leftGuess),
      ascent: finite(m.actualBoundingBoxAscent, px * 0.72),
      descent: finite(m.actualBoundingBoxDescent, 0),
      blank: s.trim() === '',
    };
  });
}

/**
 * Dispone le righe e sposta il blocco perche' il centro dell'inchiostro cada sul punto del livello: un testo con x = 0,5 e'
 * centrato davvero sull'immagine, qualunque siano il carattere, le sue spalle e la linea di base.
 */
export function textGeometry(lines: readonly string[], metrics: readonly LineMetrics[], align: TextLayer['align'], px: number): TextGeometry {
  const lineH = px * LINE_HEIGHT;
  const advance = Math.max(1, ...metrics.map((m) => m.width));
  const anchorX = align === 'left' ? -advance / 2 : align === 'right' ? advance / 2 : 0;
  let r: Rect | null = null;
  for (const [i, m] of metrics.entries()) {
    if (m.blank) continue;
    const base = i * lineH;
    const x0 = anchorX - m.left;
    const x1 = anchorX + m.right;
    const y0 = base - m.ascent;
    const y1 = base + m.descent;
    r = r ? { x0: Math.min(r.x0, x0), y0: Math.min(r.y0, y0), x1: Math.max(r.x1, x1), y1: Math.max(r.y1, y1) } : { x0, y0, x1, y1 };
  }
  // senza inchiostro (testo vuoto) si tiene l'ingombro delle righe, perche' resti qualcosa da afferrare
  const half = Math.max(advance, px * 0.5) / 2;
  const box: Rect = r ?? { x0: -half, y0: -px * 0.8, x1: half, y1: (Math.max(1, metrics.length) - 1) * lineH + px * 0.2 };
  const w = Math.max(1, box.x1 - box.x0);
  const h = Math.max(1, box.y1 - box.y0);
  const dx = -(box.x0 + box.x1) / 2;
  const dy = -(box.y0 + box.y1) / 2;
  return {
    lines: [...lines],
    px,
    anchors: lines.map((_, i) => ({ x: anchorX + dx, y: i * lineH + dy })),
    ink: { x0: -w / 2, y0: -h / 2, x1: w / 2, y1: h / 2 },
  };
}

/** Spessore del contorno in pixel (0 = niente). */
function outlineWidth(l: TextLayer, px: number): number {
  return l.outline > 0 ? Math.max(1, l.outline * px) : 0;
}

/** Ombra: spostamento e sfocatura in pixel. */
function shadowOf(px: number): { off: number; blur: number } {
  return { off: px * 0.06, blur: px * 0.18 };
}

/** Targhetta di sfondo del testo: l'inchiostro (con il contorno) piu' un margine. */
export function plateRect(g: TextGeometry, l: TextLayer): Rect {
  const o = outlineWidth(l, g.px) / 2;
  const padX = g.px * 0.3;
  const padY = g.px * 0.22;
  return { x0: g.ink.x0 - o - padX, y0: g.ink.y0 - o - padY, x1: g.ink.x1 + o + padX, y1: g.ink.y1 + o + padY };
}

/** Riquadro di selezione: quello che si vede del testo, cioe' glifi, contorno, ombra e targhetta. */
export function textSelectionRect(g: TextGeometry, l: TextLayer): Rect {
  const o = outlineWidth(l, g.px) / 2;
  const r: Rect = { x0: g.ink.x0 - o, y0: g.ink.y0 - o, x1: g.ink.x1 + o, y1: g.ink.y1 + o };
  if (l.shadow) {
    // la sfocatura si vede per circa meta' del suo valore, spostata in basso a destra
    const s = shadowOf(g.px);
    const reach = s.blur / 2;
    r.x0 = Math.min(r.x0, g.ink.x0 + s.off - reach);
    r.y0 = Math.min(r.y0, g.ink.y0 + s.off - reach);
    r.x1 = Math.max(r.x1, g.ink.x1 + s.off + reach);
    r.y1 = Math.max(r.y1, g.ink.y1 + s.off + reach);
  }
  if (l.box) {
    const p = plateRect(g, l);
    r.x0 = Math.min(r.x0, p.x0);
    r.y0 = Math.min(r.y0, p.y0);
    r.x1 = Math.max(r.x1, p.x1);
    r.y1 = Math.max(r.y1, p.y1);
  }
  const pad = Math.max(1, g.px * 0.03);
  return { x0: r.x0 - pad, y0: r.y0 - pad, x1: r.x1 + pad, y1: r.y1 + pad };
}

/** Geometria di un livello di testo (testo gia' espanso), misurata sul contesto dato. */
export function layoutText(ctx: CanvasRenderingContext2D, l: TextLayer, text: string, H: number): TextGeometry {
  const px = Math.max(1, Math.round(l.size * H));
  const lines = text.split('\n');
  return textGeometry(lines, measureLines(ctx, l, lines, px), l.align, px);
}

/**
 * Ingombro di un livello in pixel dell'immagine, per selezione e maniglie: perno (cx, cy, dove il livello ruota), lati del
 * riquadro e spostamento (ox, oy) del suo centro dal perno, nel riferimento ruotato del livello.
 */
export interface LayerBox {
  cx: number;
  cy: number;
  w: number;
  h: number;
  ox: number;
  oy: number;
}

export function layerBox(ctx: CanvasRenderingContext2D, l: Layer, W: number, H: number, tokens: TokenContext): LayerBox {
  if (l.kind === 'shape') return { cx: l.x * W, cy: l.y * H, w: l.w * W, h: l.h * H, ox: 0, oy: 0 };
  const r = textSelectionRect(layoutText(ctx, l, expandTokens(l.text, tokens), H), l);
  return { cx: l.x * W, cy: l.y * H, w: r.x1 - r.x0, h: r.y1 - r.y0, ox: (r.x0 + r.x1) / 2, oy: (r.y0 + r.y1) / 2 };
}

/** Punto (lx, ly) del riquadro, preso dal suo centro nel riferimento ruotato, in pixel dell'immagine. */
export function boxPoint(b: LayerBox, deg: number, lx: number, ly: number): { x: number; y: number } {
  const a = (deg * Math.PI) / 180;
  const x = b.ox + lx;
  const y = b.oy + ly;
  return { x: b.cx + x * Math.cos(a) - y * Math.sin(a), y: b.cy + x * Math.sin(a) + y * Math.cos(a) };
}

function drawText(ctx: CanvasRenderingContext2D, l: TextLayer, W: number, H: number, tokens: TokenContext): void {
  ctx.save();
  const g = layoutText(ctx, l, expandTokens(l.text, tokens), H);
  ctx.translate(l.x * W, l.y * H);
  ctx.rotate((l.rotation * Math.PI) / 180);
  setTextStyle(ctx, l, g.px);
  if (l.box) {
    const p = plateRect(g, l);
    ctx.fillStyle = hexAlpha(l.boxColor, l.boxOpacity);
    ctx.beginPath();
    ctx.roundRect(p.x0, p.y0, p.x1 - p.x0, p.y1 - p.y0, g.px * 0.25);
    ctx.fill();
  }
  const fill = paint(ctx, l.fill, g.ink.x0, g.ink.y0, g.ink.x1 - g.ink.x0, g.ink.y1 - g.ink.y0);
  const ow = outlineWidth(l, g.px);
  const sh = shadowOf(g.px);
  g.lines.forEach((line, i) => {
    const at = g.anchors[i];
    if (!at) return;
    if (l.shadow) {
      ctx.save();
      ctx.shadowColor = hexAlpha(l.shadowColor, 0.85);
      ctx.shadowBlur = sh.blur;
      ctx.shadowOffsetX = sh.off;
      ctx.shadowOffsetY = sh.off;
      ctx.fillStyle = l.shadowColor;
      ctx.fillText(line, at.x, at.y);
      ctx.restore();
    }
    if (ow > 0) {
      ctx.lineJoin = 'round';
      ctx.miterLimit = 2;
      ctx.strokeStyle = l.outlineColor;
      ctx.lineWidth = ow;
      ctx.strokeText(line, at.x, at.y);
    }
    ctx.fillStyle = fill;
    ctx.fillText(line, at.x, at.y);
  });
  ctx.restore();
}

const BARS = ['#c0c0c0', '#c0c000', '#00c0c0', '#00c000', '#c000c0', '#c00000', '#0000c0'];

function drawShape(ctx: CanvasRenderingContext2D, l: ShapeLayer, W: number, H: number): void {
  const w = l.w * W;
  const h = l.h * H;
  ctx.save();
  ctx.globalAlpha = l.opacity;
  ctx.translate(l.x * W, l.y * H);
  ctx.rotate((l.rotation * Math.PI) / 180);
  const x = -w / 2;
  const y = -h / 2;
  const sw = Math.max(1, l.stroke * H);
  if (l.shape === 'bars') {
    BARS.forEach((c, i) => {
      ctx.fillStyle = c;
      ctx.fillRect(x + (w * i) / BARS.length, y, w / BARS.length + 0.5, h);
    });
  } else if (l.shape === 'ellipse') {
    ctx.beginPath();
    ctx.ellipse(0, 0, Math.abs(w / 2), Math.abs(h / 2), 0, 0, Math.PI * 2);
    if (l.filled) {
      ctx.fillStyle = paint(ctx, l.fill, x, y, w, h);
      ctx.fill();
    }
    if (l.stroke > 0) {
      ctx.strokeStyle = l.strokeColor;
      ctx.lineWidth = sw;
      ctx.stroke();
    }
  } else {
    const frame = l.shape === 'frame';
    if (l.filled && !frame) {
      ctx.fillStyle = paint(ctx, l.fill, x, y, w, h);
      ctx.fillRect(x, y, w, h);
    }
    if (l.stroke > 0 || frame) {
      ctx.strokeStyle = frame && l.fill.kind === 'gradient' ? paint(ctx, l.fill, x, y, w, h) : l.strokeColor;
      ctx.lineWidth = sw;
      // la cornice sta tutta dentro il riquadro
      ctx.strokeRect(x + sw / 2, y + sw / 2, w - sw, h - sw);
    }
  }
  ctx.restore();
}

/** Disegna la composizione intera sul canvas (alla risoluzione del modo): e' anche l'immagine che si trasmette. */
export function renderComposition(ctx: CanvasRenderingContext2D, comp: Composition, bg: BgSource | null, tokens: TokenContext): void {
  const W = ctx.canvas.width;
  const H = ctx.canvas.height;
  const b = comp.background;
  ctx.save();
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = 'high';
  ctx.fillStyle = b.color;
  ctx.fillRect(0, 0, W, H);
  if (bg) {
    const s = sourceSize(bg);
    const r = placeImage(s.w, s.h, W, H, b.fit, b.zoom, b.panX, b.panY);
    ctx.drawImage(bg, r.x, r.y, r.w, r.h);
    const a = b.adjust;
    if (a.brightness !== 0 || a.contrast !== 0 || a.saturation !== 0) {
      const data = ctx.getImageData(0, 0, W, H);
      applyAdjust(data.data, a);
      ctx.putImageData(data, 0, 0);
    }
  }
  for (const l of comp.layers) {
    if (l.hidden) continue;
    if (l.kind === 'text') drawText(ctx, l, W, H, tokens);
    else drawShape(ctx, l, W, H);
  }
  ctx.restore();
}

/** Massimo lato tenuto in memoria per una foto: le foto enormi si riducono una volta sola (fluide da spostare e ingrandire). */
const MAX_SIDE = 2400;

/**
 * Legge un file immagine (JPEG, PNG, WebP, GIF primo fotogramma, BMP) rispettando l'orientamento EXIF e lo riduce se e' enorme.
 * `null` se il browser non sa leggerlo.
 */
export async function loadImageFile(file: Blob): Promise<HTMLCanvasElement | null> {
  let bitmap: ImageBitmap;
  try {
    bitmap = await createImageBitmap(file, { imageOrientation: 'from-image' });
  } catch {
    return null;
  }
  const scale = Math.min(1, MAX_SIDE / Math.max(bitmap.width, bitmap.height));
  const canvas = document.createElement('canvas');
  canvas.width = Math.max(1, Math.round(bitmap.width * scale));
  canvas.height = Math.max(1, Math.round(bitmap.height * scale));
  const ctx = canvas.getContext('2d');
  if (!ctx) return null;
  ctx.imageSmoothingQuality = 'high';
  ctx.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  bitmap.close();
  return canvas;
}
