import { describe, expect, it } from 'vitest';
import { distanceKm, gridCenter, gridOfMessage } from './geo';

describe('gridCenter', () => {
  it('returns the center of a 4-character locator', () => {
    const c = gridCenter('JN45');
    expect(c?.lat).toBeCloseTo(45.5, 5);
    expect(c?.lon).toBeCloseTo(9, 5);
  });

  it('is more precise with 6 characters and ignores case', () => {
    const c = gridCenter('jn45xk');
    expect(c?.lat).toBeCloseTo(45.5 - 0.5 + (10 + 0.5) / 24, 3);
    expect(gridCenter('JN45XK')).toEqual(c);
  });

  it('rejects anything that is not a locator', () => {
    for (const bad of ['', 'ZZ99', 'JN4', 'JN45X', '1234', 'JN4A']) expect(gridCenter(bad)).toBeNull();
  });
});

describe('distanceKm', () => {
  it('is zero on the same point and symmetric', () => {
    const a = gridCenter('JN45')!;
    const b = gridCenter('PM95')!;
    expect(distanceKm(a, a)).toBe(0);
    expect(distanceKm(a, b)).toBe(distanceKm(b, a));
  });

  it('gives a plausible Europe to Japan distance', () => {
    const km = distanceKm(gridCenter('JN45')!, gridCenter('PM95')!);
    expect(km).toBeGreaterThan(9000);
    expect(km).toBeLessThan(10000);
  });
});

describe('gridOfMessage', () => {
  it('reads the locator at the end of a CQ or a call', () => {
    expect(gridOfMessage('CQ DX JA1XZF QM05')).toBe('QM05');
    expect(gridOfMessage('H44MS OH3XR KP11')).toBe('KP11');
  });

  it('does not take a signal report or RR73 for a locator', () => {
    expect(gridOfMessage('UX7QV PW2DSY RR73')).toBeNull();
    expect(gridOfMessage('VU2FR R2BQM -18')).toBeNull();
    expect(gridOfMessage('')).toBeNull();
  });
});
