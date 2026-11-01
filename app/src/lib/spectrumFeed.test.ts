import { describe, expect, it } from 'vitest';
import { cropBins, spanFromPassband } from './spectrumFeed';

describe('waterfall span', () => {
  it('follows the radio filter, 3000 Hz when unknown', () => {
    expect(spanFromPassband(null)).toBe(3000);
    expect(spanFromPassband(0)).toBe(3000);
    expect(spanFromPassband(3800)).toBe(4000);
    expect(spanFromPassband(2400)).toBe(2600);
    expect(spanFromPassband(200)).toBe(1000);
    expect(spanFromPassband(9000)).toBe(5000);
  });

  it('keeps only the bins up to the span', () => {
    const bins = new Uint8Array(2048);
    expect(cropBins(bins, 0, 5000, 4000).length).toBe(1638);
    expect(cropBins(bins, 0, 5000, 5000).length).toBe(2048);
    expect(cropBins(bins, 0, 3000, 3000).length).toBe(2048);
  });
});
