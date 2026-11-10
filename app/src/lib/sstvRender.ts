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

/** Ingombro di un testo (in pixel dell'immagine): larghezza e altezza del blocco, senza rotazione. */
export function measureText(ctx: CanvasRenderingContext2D, l: TextLayer, text: string, H: number): { w: number; h: number; lines: string[]; px: number } {
  const px = l.size * H;
  ctx.font = fontString(l, px);
  const lines = text.split('\n');
  const w = Math.max(1, ...lines.map((s) => ctx.measureText(s).width));
  return { w, h: px * 1.18 * lines.length, lines, px };
}

/** Ingombro di un livello (centro, lati) in pixel, per selezione e maniglie. */
export function layerBox(ctx: CanvasRenderingContext2D, l: Layer, W: number, H: number, tokens: TokenContext): { cx: number; cy: number; w: number; h: number } {
  if (l.kind === 'shape') return { cx: l.x * W, cy: l.y * H, w: l.w * W, h: l.h * H };
  const m = measureText(ctx, l, expandTokens(l.text, tokens), H);
  const pad = l.box ? m.px * 0.3 : m.px * 0.1;
  return { cx: l.x * W, cy: l.y * H, w: m.w + pad * 2, h: m.h + pad * 2 };
}

function drawText(ctx: CanvasRenderingContext2D, l: TextLayer, W: number, H: number, tokens: TokenContext): void {
  const text = expandTokens(l.text, tokens);
  const m = measureText(ctx, l, text, H);
  ctx.save();
  ctx.translate(l.x * W, l.y * H);
  ctx.rotate((l.rotation * Math.PI) / 180);
  ctx.font = fontString(l, m.px);
  ctx.textBaseline = 'middle';
  ctx.textAlign = l.align;
  const lineH = m.px * 1.18;
  const top = -m.h / 2 + lineH / 2;
  const anchorX = l.align === 'left' ? -m.w / 2 : l.align === 'right' ? m.w / 2 : 0;
  if (l.box) {
    const pad = m.px * 0.3;
    ctx.fillStyle = hexAlpha(l.boxColor, l.boxOpacity);
    const r = Math.min(pad, m.px * 0.25);
    ctx.beginPath();
    ctx.roundRect(-m.w / 2 - pad, -m.h / 2 - pad * 0.6, m.w + pad * 2, m.h + pad * 1.2, r);
    ctx.fill();
  }
  const fill = paint(ctx, l.fill, -m.w / 2, -m.h / 2, m.w, m.h);
  m.lines.forEach((line, i) => {
    const y = top + i * lineH;
    if (l.shadow) {
      ctx.save();
      ctx.shadowColor = hexAlpha(l.shadowColor, 0.85);
      ctx.shadowBlur = m.px * 0.18;
      ctx.shadowOffsetX = m.px * 0.06;
      ctx.shadowOffsetY = m.px * 0.06;
      ctx.fillStyle = l.shadowColor;
      ctx.fillText(line, anchorX, y);
      ctx.restore();
    }
    if (l.outline > 0) {
      ctx.lineJoin = 'round';
      ctx.miterLimit = 2;
      ctx.strokeStyle = l.outlineColor;
      ctx.lineWidth = Math.max(1, l.outline * m.px);
      ctx.strokeText(line, anchorX, y);
    }
    ctx.fillStyle = fill;
    ctx.fillText(line, anchorX, y);
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
