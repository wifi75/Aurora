import { describe, expect, it } from 'vitest';
import { bearingDeg, distanceKm, gridCenter, gridOfMessage, greatCircle, latLonToGrid } from './geo';

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

describe('bearing, great circle and locator of a point', () => {
  const rome = { lat: 41.9, lon: 12.5 };
  const tokyo = { lat: 35.7, lon: 139.7 };

  it('gives the initial bearing between two places', () => {
    expect(bearingDeg({ lat: 0, lon: 0 }, { lat: 10, lon: 0 })).toBe(0);
    expect(bearingDeg({ lat: 0, lon: 0 }, { lat: 0, lon: 10 })).toBe(90);
    expect(bearingDeg({ lat: 0, lon: 0 }, { lat: -10, lon: 0 })).toBe(180);
    expect(bearingDeg({ lat: 0, lon: 0 }, { lat: 0, lon: -10 })).toBe(270);
    expect(bearingDeg(rome, tokyo)).toBeGreaterThan(35);
    expect(bearingDeg(rome, tokyo)).toBeLessThan(45);
  });

  it('draws the short way: it starts and ends on the points and bends towards the pole', () => {
    const line = greatCircle(rome, tokyo, 24);
    expect(line).toHaveLength(25);
    expect(line[0]).toEqual({ lat: rome.lat, lon: rome.lon });
    expect(line.at(-1)?.lat).toBeCloseTo(tokyo.lat, 5);
    expect(line.at(-1)?.lon).toBeCloseTo(tokyo.lon, 5);
    expect(Math.max(...line.map((p) => p.lat))).toBeGreaterThan(50);
  });

  it('keeps the longitude continuous across the date line', () => {
    const line = greatCircle({ lat: 35, lon: 170 }, { lat: 35, lon: -170 }, 20);
    for (let i = 1; i < line.length; i++) expect(Math.abs(line[i]!.lon - line[i - 1]!.lon)).toBeLessThan(5);
    expect(line.at(-1)?.lon).toBeGreaterThan(180);
  });

  it('handles two identical points without dividing by zero', () => {
    expect(greatCircle(rome, rome)).toHaveLength(2);
  });

  it('finds the six character locator of a place', () => {
    expect(latLonToGrid(41.9, 12.5)).toBe('JN61gv');
    expect(latLonToGrid(-90, -180)).toBe('AA00aa');
    expect(latLonToGrid(89.99, 179.99)).toBe('RR99xx');
    expect(latLonToGrid(41.9, 12.5 + 360)).toBe('JN61gv');
  });

  it('agrees with the locator reader', () => {
    const p = gridCenter('JN45te')!;
    expect(latLonToGrid(p.lat, p.lon)).toBe('JN45te');
  });
});
