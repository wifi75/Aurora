import { describe, expect, it } from 'vitest';
import { decodeGrayRows, fitScale, followScrollTop, grayIntoRgba, stepZoom } from './wefax';

describe('wefax helpers', () => {
  it('fits the picture to the available width without upscaling too much', () => {
    expect(fitScale(904, 1809)).toBeCloseTo(0.4997, 3);
    expect(fitScale(1809, 1809)).toBe(1);
    expect(fitScale(5000, 904)).toBe(2);
    expect(fitScale(0, 904)).toBe(1);
    expect(fitScale(10, 1809)).toBe(0.05);
  });

  it('steps the zoom up and down and stops at the ends', () => {
    expect(stepZoom(1, 1)).toBe(1.5);
    expect(stepZoom(1, -1)).toBe(0.75);
    // da un valore intermedio (adattamento) si va al gradino piu' vicino nel verso scelto
    expect(stepZoom(0.4997, 1)).toBe(0.5);
    expect(stepZoom(0.4997, -1)).toBe(0.33);
    expect(stepZoom(4, 1)).toBe(4);
    expect(stepZoom(0.1, -1)).toBe(0.1);
  });

  it('scrolls so the last active row stays at the bottom, never negative', () => {
    expect(followScrollTop(1000, 1, 600)).toBe(408);
    expect(followScrollTop(100, 0.5, 600)).toBe(0);
  });

  it('turns gray bytes into opaque RGBA from the given row', () => {
    const out = new Uint8ClampedArray(2 * 2 * 4);
    grayIntoRgba(new Uint8Array([10, 200]), out, 2, 1);
    expect([...out.slice(0, 8)]).toEqual([0, 0, 0, 0, 0, 0, 0, 0]);
    expect([...out.slice(8, 16)]).toEqual([10, 10, 10, 255, 200, 200, 200, 255]);
  });

  it('decodes base64 gray rows, keeping only whole rows', () => {
    // 2 righe da 3 pixel piu' un byte avanzato
    const b64 = btoa(String.fromCharCode(0, 128, 255, 1, 2, 3, 9));
    const { rgba, rows } = decodeGrayRows(b64, 3);
    expect(rows).toBe(2);
    expect(rgba.length).toBe(2 * 3 * 4);
    expect([...rgba.slice(4, 8)]).toEqual([128, 128, 128, 255]);
    expect([...rgba.slice(20, 24)]).toEqual([3, 3, 3, 255]);
  });
});
