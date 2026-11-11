import { describe, expect, it } from 'vitest';
import {
  angleFrom,
  applyAdjust,
  duplicateLayer,
  emptyComposition,
  expandTokens,
  formatDuration,
  historyPush,
  historyRedo,
  historyStart,
  historyUndo,
  moveLayer,
  parseTemplates,
  placeImage,
  pointInRotatedRect,
  rgbIntoRgba,
  textLayer,
} from './sstv';

describe('placeImage', () => {
  it('cover fills the target and crops the excess at the centre', () => {
    const r = placeImage(4000, 3000, 320, 256, 'cover');
    expect(r.h).toBeCloseTo(256);
    expect(r.w).toBeCloseTo(341.33, 1);
    expect(r.x).toBeCloseTo(-10.67, 1);
    expect(r.y).toBeCloseTo(0);
  });

  it('contain shows the whole photo with bands', () => {
    const r = placeImage(1000, 1000, 320, 256, 'contain');
    expect(r).toEqual({ x: 32, y: 0, w: 256, h: 256 });
  });

  it('pan moves only as far as the photo reaches', () => {
    const left = placeImage(4000, 3000, 320, 256, 'cover', 1, -1, 0);
    expect(left.x).toBeCloseTo(0);
    const right = placeImage(4000, 3000, 320, 256, 'cover', 1, 1, 0);
    expect(right.x + right.w).toBeCloseTo(320);
    // in verticale la foto non sporge: lo spostamento non fa niente
    expect(placeImage(4000, 3000, 320, 256, 'cover', 1, 0, 1).y).toBeCloseTo(0);
  });

  it('zoom enlarges around the centre', () => {
    const r = placeImage(320, 256, 320, 256, 'cover', 2);
    expect(r).toEqual({ x: -160, y: -128, w: 640, h: 512 });
  });
});

describe('expandTokens', () => {
  const now = new Date(Date.UTC(2026, 10, 10, 15, 4));
  it('fills the tokens case-insensitively and leaves unknown ones', () => {
    expect(expandTokens('{call} de {MYCALL} {Rst} {GRID} {x}', { mycall: 'iu3abc', call: 'k1abc', rst: '595', grid: 'IO91wm', now })).toBe('K1ABC de IU3ABC 595 IO91wm {x}');
  });
  it('writes date and time in UTC', () => {
    expect(expandTokens('{DATE} {TIME}', { now })).toBe('2026-11-10 15:04 UTC');
  });
  it('uses 595 as default report', () => {
    expect(expandTokens('UR {RST}', {})).toBe('UR 595');
  });
});

describe('pixels', () => {
  it('copies RGB rows into RGBA at the right row', () => {
    const out = new Uint8ClampedArray(2 * 2 * 4);
    rgbIntoRgba(new Uint8Array([1, 2, 3, 4, 5, 6]), out, 2, 1);
    expect(Array.from(out.slice(8))).toEqual([1, 2, 3, 255, 4, 5, 6, 255]);
    expect(out[3]).toBe(0);
  });

  it('adjust: neutral does nothing, brightness lifts, zero saturation gives grey', () => {
    const px = new Uint8ClampedArray([200, 100, 50, 255]);
    applyAdjust(px, { brightness: 0, contrast: 0, saturation: 0 });
    expect(Array.from(px)).toEqual([200, 100, 50, 255]);
    applyAdjust(px, { brightness: 10, contrast: 0, saturation: 0 });
    expect(px[0]).toBe(213);
    const grey = new Uint8ClampedArray([200, 100, 50, 255]);
    applyAdjust(grey, { brightness: 0, contrast: 0, saturation: -100 });
    expect(grey[0]).toBe(grey[1]);
    expect(grey[1]).toBe(grey[2]);
  });
});

describe('layers and history', () => {
  it('moves, duplicates and keeps order', () => {
    const a = textLayer('A');
    const b = textLayer('B');
    expect(moveLayer([a, b], a.id, 1).map((l) => (l.kind === 'text' ? l.text : ''))).toEqual(['B', 'A']);
    expect(moveLayer([a, b], a.id, -1).map((l) => (l.kind === 'text' ? l.text : ''))).toEqual(['A', 'B']);
    const d = duplicateLayer([a, b], a.id);
    expect(d).toHaveLength(3);
    expect(d[1]?.id).not.toBe(a.id);
  });

  it('undo and redo walk the snapshots', () => {
    const c0 = emptyComposition();
    const c1 = { ...c0, layers: [textLayer('x')] };
    let h = historyPush(historyStart(c0), c1);
    h = historyUndo(h);
    expect(h.present).toBe(c0);
    h = historyRedo(h);
    expect(h.present).toBe(c1);
    expect(historyRedo(h)).toBe(h);
  });

  it('hit test follows the rotation', () => {
    expect(pointInRotatedRect(0.5, 0.5, 0.5, 0.5, 0.4, 0.1, 0)).toBe(true);
    expect(pointInRotatedRect(0.5, 0.65, 0.5, 0.5, 0.4, 0.1, 0)).toBe(false);
    expect(pointInRotatedRect(0.5, 0.65, 0.5, 0.5, 0.4, 0.1, 90)).toBe(true);
    expect(angleFrom(0, 0, 1, 0)).toBe(90);
    expect(angleFrom(0, 0, 0, -1)).toBe(0);
  });

  it('templates: only well-formed ones survive', () => {
    const good = { name: 'CQ', composition: emptyComposition() };
    expect(parseTemplates(JSON.stringify([good, { name: 3 }, 'x']))).toHaveLength(1);
    expect(parseTemplates('not json')).toEqual([]);
    expect(parseTemplates(null)).toEqual([]);
  });

  it('formats durations', () => {
    expect(formatDuration(36)).toBe('36 s');
    expect(formatDuration(114.3)).toBe('1:54');
  });
});
