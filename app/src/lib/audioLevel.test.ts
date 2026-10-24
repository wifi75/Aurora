import { describe, expect, it } from 'vitest';
import { meterPercent, toDb } from './audioLevel';

describe('audio level scale', () => {
  it('maps amplitude to dBFS with a floor', () => {
    expect(toDb(1)).toBeCloseTo(0);
    expect(toDb(0.5)).toBeCloseTo(-6.02, 1);
    expect(toDb(0)).toBe(-60);
    expect(toDb(0.00001)).toBe(-60);
    expect(toDb(Number.NaN)).toBe(-60);
  });

  it('puts full scale at the top and silence at the bottom', () => {
    expect(meterPercent(1)).toBe(100);
    expect(meterPercent(0)).toBe(0);
    expect(meterPercent(0.1)).toBe(67);
  });
});
