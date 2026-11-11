import { describe, expect, it } from 'vitest';
import { emptyComposition, textLayer } from './sstv';
import { boxPoint, fontString, layerBox, measureLines, renderComposition, textGeometry, textSelectionRect, type LineMetrics } from './sstvRender';

/** Inchiostro finto di ogni carattere: avanzamento 10 px su 40 px, glifi stretti a sinistra (spalla destra larga). */
const ADV = 0.25;
const LSB = 0.05;
const RSB = 0.15;
const ASC = 0.7;
const DESC = 0.1;

interface Call {
  op: string;
  args: unknown[];
  font: string;
  align: string;
  baseline: string;
}

/** Contesto 2D finto: misura con le regole qui sopra e registra le chiamate di disegno con lo stato del momento. */
function fakeCtx(W = 320, H = 256) {
  const calls: Call[] = [];
  const ctx = {
    canvas: { width: W, height: H },
    font: '10px sans-serif',
    textAlign: 'start',
    textBaseline: 'alphabetic',
    fillStyle: '',
    strokeStyle: '',
    lineWidth: 1,
    lineJoin: 'miter',
    miterLimit: 10,
    shadowColor: '',
    shadowBlur: 0,
    shadowOffsetX: 0,
    shadowOffsetY: 0,
    imageSmoothingEnabled: true,
    imageSmoothingQuality: 'low',
    measureText(s: string) {
      const px = Number(/(\d+)px/.exec(this.font)?.[1] ?? 10);
      const width = s.length * ADV * px;
      const inkL = s.length ? LSB * px : 0;
      const inkR = s.length ? width - RSB * px : 0;
      // come il canvas: lati misurati dal punto di ancoraggio secondo textAlign
      const anchor = this.textAlign === 'center' ? width / 2 : this.textAlign === 'right' ? width : 0;
      return {
        width,
        actualBoundingBoxLeft: anchor - inkL,
        actualBoundingBoxRight: inkR - anchor,
        actualBoundingBoxAscent: s.length ? ASC * px : 0,
        actualBoundingBoxDescent: s.length ? DESC * px : 0,
      };
    },
    save() {},
    restore() {},
    translate(...args: unknown[]) {
      calls.push({ op: 'translate', args, font: ctx.font, align: ctx.textAlign, baseline: ctx.textBaseline });
    },
    rotate() {},
    fillRect() {},
    beginPath() {},
    roundRect(...args: unknown[]) {
      calls.push({ op: 'roundRect', args, font: ctx.font, align: ctx.textAlign, baseline: ctx.textBaseline });
    },
    fill() {},
    createLinearGradient() {
      return { addColorStop() {} };
    },
    fillText(...args: unknown[]) {
      calls.push({ op: 'fillText', args, font: ctx.font, align: ctx.textAlign, baseline: ctx.textBaseline });
    },
    strokeText(...args: unknown[]) {
      calls.push({ op: 'strokeText', args, font: ctx.font, align: ctx.textAlign, baseline: ctx.textBaseline });
    },
  };
  return { ctx: ctx as unknown as CanvasRenderingContext2D, calls };
}

const NONE = { outline: 0, shadow: false, box: false };
const tokens = { mycall: 'IU3ABC', grid: 'IO91wm' };

describe('textGeometry', () => {
  const m = (over: Partial<LineMetrics> = {}): LineMetrics => ({ width: 100, left: 40, right: 60, ascent: 70, descent: 5, blank: false, ...over });

  it('centres the ink, not the advance or the baseline, on the layer point', () => {
    const g = textGeometry(['IU3ABC'], [m()], 'center', 100);
    expect(g.ink).toEqual({ x0: -50, y0: -37.5, x1: 50, y1: 37.5 });
    // glifi da -40 a +60 rispetto all'ancoraggio: si sposta di 10 a sinistra; da -70 a +5 sulla base: si scende di 32,5
    expect(g.anchors[0]?.x).toBeCloseTo(-10);
    expect(g.anchors[0]?.y).toBeCloseTo(32.5);
  });

  it('left and right alignment keep the ink centred too', () => {
    for (const align of ['left', 'right'] as const) {
      const left = align === 'left' ? -5 : 95;
      const g = textGeometry(['A'], [m({ left, right: 90 - left })], align, 100);
      const at = g.anchors[0];
      expect(at).toBeDefined();
      if (!at) continue;
      expect(at.x - left).toBeCloseTo(g.ink.x0);
      expect(at.x - left + 90).toBeCloseTo(g.ink.x1);
    }
  });

  it('several lines: the union of the ink is centred and lines keep their spacing', () => {
    const g = textGeometry(['CQ', 'DE'], [m(), m({ width: 50, left: 20, right: 20 })], 'center', 100);
    const [a, b] = g.anchors;
    expect(b && a ? b.y - a.y : 0).toBeCloseTo(118);
    expect(g.ink.x1 - g.ink.x0).toBeCloseTo(100);
    expect(g.ink.y1 - g.ink.y0).toBeCloseTo(118 + 70 + 5);
    expect(g.ink.x0 + g.ink.x1).toBeCloseTo(0);
    expect(g.ink.y0 + g.ink.y1).toBeCloseTo(0);
  });

  it('empty text still has a box to grab', () => {
    const g = textGeometry([''], [m({ width: 0, left: 0, right: 0, ascent: 0, descent: 0, blank: true })], 'center', 40);
    expect(g.ink.x1 - g.ink.x0).toBeGreaterThan(10);
    expect(g.ink.y1 - g.ink.y0).toBeCloseTo(40);
  });
});

describe('measureLines', () => {
  it('measures with the same font, alignment and baseline used to draw', () => {
    const { ctx } = fakeCtx();
    const l = textLayer('IU3ABC', { bold: true });
    const [r] = measureLines(ctx, l, ['IU3ABC'], 40);
    expect(ctx.font).toBe(fontString(l, 40));
    expect(ctx.textAlign).toBe('center');
    expect(ctx.textBaseline).toBe('alphabetic');
    expect(r).toEqual({ width: 60, left: 28, right: 24, ascent: 28, descent: 4, blank: false });
  });
});

describe('layerBox', () => {
  it('the selection wraps the glyphs tightly around the layer point', () => {
    const { ctx } = fakeCtx();
    const l = textLayer('IU3ABC', { ...NONE, size: 40 / 256 });
    const b = layerBox(ctx, l, 320, 256, tokens);
    expect(b.cx).toBe(160);
    expect(b.cy).toBe(128);
    // inchiostro 52 x 32 px, piu' un margine di 1,2 px per lato
    expect(b.w).toBeCloseTo(52 + 2.4);
    expect(b.h).toBeCloseTo(32 + 2.4);
    expect(b.ox).toBeCloseTo(0);
    expect(b.oy).toBeCloseTo(0);
  });

  it('outline, shadow and plate widen the box on the sides where they show', () => {
    const { ctx } = fakeCtx();
    const plain = layerBox(ctx, textLayer('CQ SSTV', { ...NONE, size: 0.2 }), 320, 256, tokens);
    const outlined = layerBox(ctx, textLayer('CQ SSTV', { ...NONE, size: 0.2, outline: 0.1 }), 320, 256, tokens);
    expect(outlined.w - plain.w).toBeCloseTo(51 * 0.1);
    expect(outlined.ox).toBeCloseTo(0);
    const shadowed = layerBox(ctx, textLayer('CQ SSTV', { ...NONE, size: 0.2, shadow: true }), 320, 256, tokens);
    expect(shadowed.w).toBeGreaterThan(plain.w);
    // l'ombra cade in basso a destra: il riquadro si allarga da quella parte
    expect(shadowed.ox).toBeGreaterThan(0);
    expect(shadowed.oy).toBeGreaterThan(0);
    const plated = layerBox(ctx, textLayer('CQ SSTV', { ...NONE, size: 0.2, box: true }), 320, 256, tokens);
    expect(plated.w - plain.w).toBeCloseTo(2 * 51 * 0.3);
  });

  it('expands the tokens before measuring', () => {
    const { ctx } = fakeCtx();
    const a = layerBox(ctx, textLayer('DE {MYCALL}', { ...NONE }), 320, 256, tokens);
    const b = layerBox(ctx, textLayer('DE IU3ABC', { ...NONE }), 320, 256, tokens);
    expect(a).toEqual(b);
  });
});

describe('boxPoint', () => {
  it('follows the box offset and the rotation around the pivot', () => {
    const b = { cx: 100, cy: 50, w: 40, h: 20, ox: 3, oy: 4 };
    expect(boxPoint(b, 0, 20, 10)).toEqual({ x: 123, y: 64 });
    const p = boxPoint(b, 90, 20, 0);
    expect(p.x).toBeCloseTo(96);
    expect(p.y).toBeCloseTo(73);
  });
});

describe('renderComposition', () => {
  it('draws the glyphs exactly inside the selection box, centred on the image', () => {
    const W = 320;
    const H = 256;
    const { ctx, calls } = fakeCtx(W, H);
    const comp = emptyComposition();
    const layers = [
      textLayer('CQ SSTV', { y: 0.2, size: 0.2 }),
      textLayer('DE {MYCALL}', { y: 0.72, size: 0.15, font: 'Bebas Neue' }),
      textLayer('{GRID}', { y: 0.88, size: 0.08, font: 'Orbitron', box: true }),
    ];
    renderComposition(ctx, { ...comp, layers }, null, tokens);
    const texts = calls.filter((c) => c.op === 'fillText');
    const origins = calls.filter((c) => c.op === 'translate');
    expect(origins).toHaveLength(layers.length);
    layers.forEach((l, i) => {
      const origin = origins[i]?.args as [number, number];
      expect(origin[0]).toBeCloseTo(W / 2);
      // l'ultima fillText del livello e' il riempimento (prima viene l'ombra)
      const fill = texts[i * 2 + 1];
      expect(fill).toBeDefined();
      if (!fill) return;
      const [text, x, y] = fill.args as [string, number, number];
      expect(fill.font).toBe(fontString(l, Math.round(l.size * H)));
      expect(fill.align).toBe('center');
      expect(fill.baseline).toBe('alphabetic');
      // inchiostro della riga disegnata, come lo misura lo stesso contesto
      ctx.font = fill.font;
      ctx.textAlign = 'center';
      const m = ctx.measureText(text);
      const inkX0 = x - m.actualBoundingBoxLeft;
      const inkX1 = x + m.actualBoundingBoxRight;
      const inkY0 = y - m.actualBoundingBoxAscent;
      const inkY1 = y + m.actualBoundingBoxDescent;
      // centrato sul punto del livello (x = 0,5: centro dell'immagine)
      expect((inkX0 + inkX1) / 2).toBeCloseTo(0);
      expect((inkY0 + inkY1) / 2).toBeCloseTo(0);
      // e tutto dentro il riquadro di selezione
      const b = layerBox(ctx, l, W, H, tokens);
      expect(inkX0).toBeGreaterThanOrEqual(b.ox - b.w / 2);
      expect(inkX1).toBeLessThanOrEqual(b.ox + b.w / 2);
      expect(inkY0).toBeGreaterThanOrEqual(b.oy - b.h / 2);
      expect(inkY1).toBeLessThanOrEqual(b.oy + b.h / 2);
    });
  });

  it('the plate drawn behind the text is the one inside the selection box', () => {
    const { ctx, calls } = fakeCtx();
    const l = textLayer('{GRID}', { ...NONE, y: 0.88, size: 0.08, font: 'Orbitron', box: true });
    renderComposition(ctx, { ...emptyComposition(), layers: [l] }, null, tokens);
    const plate = calls.find((c) => c.op === 'roundRect')?.args as [number, number, number, number];
    const b = layerBox(ctx, l, 320, 256, tokens);
    expect(plate[0]).toBeGreaterThanOrEqual(b.ox - b.w / 2);
    expect(plate[0] + plate[2]).toBeLessThanOrEqual(b.ox + b.w / 2);
    expect(plate[0] + plate[2] / 2).toBeCloseTo(0);
    expect(plate[1] + plate[3] / 2).toBeCloseTo(0);
  });
});

describe('textSelectionRect', () => {
  it('without decorations it is the ink plus a small margin', () => {
    const g = textGeometry(['A'], [{ width: 10, left: 4, right: 4, ascent: 8, descent: 0, blank: false }], 'center', 10);
    const r = textSelectionRect(g, textLayer('A', NONE));
    expect(r).toEqual({ x0: -5, y0: -5, x1: 5, y1: 5 });
  });
});
