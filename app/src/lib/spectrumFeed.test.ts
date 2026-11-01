import { describe, expect, it } from 'vitest';
import { emitSpectrum, registerSpectrum, spanFromPassband, visibleRange } from './spectrumFeed';

describe('waterfall span', () => {
  it('follows the radio filter, 3000 Hz when unknown', () => {
    expect(spanFromPassband(null)).toBe(3000);
    expect(spanFromPassband(0)).toBe(3000);
    expect(spanFromPassband(3800)).toBe(4000);
    expect(spanFromPassband(2400)).toBe(2600);
    expect(spanFromPassband(200)).toBe(1000);
    expect(spanFromPassband(9000)).toBe(5000);
  });

  it('hands every listener the whole row with its scale in Hz', () => {
    const seen: Array<[number, number, number]> = [];
    const off = registerSpectrum((bins, lo, hi) => seen.push([bins.length, lo, hi]));
    emitSpectrum(new Uint8Array(6827), 0, 5000);
    off();
    emitSpectrum(new Uint8Array(10), 0, 5000);
    expect(seen).toEqual([[6827, 0, 5000]]);
  });

  it('shows from Start to the filter span, never less than 500 Hz', () => {
    expect(visibleRange(0, 3000)).toEqual({ from: 0, to: 3000 });
    expect(visibleRange(500, 3000)).toEqual({ from: 500, to: 3000 });
    expect(visibleRange(2800, 3000)).toEqual({ from: 2800, to: 3800 });
    expect(visibleRange(4900, 3000)).toEqual({ from: 4500, to: 5000 });
  });
});
