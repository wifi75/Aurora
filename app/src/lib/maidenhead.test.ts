import { describe, expect, it } from 'vitest';
import { gridToLatLon, isGrid, squareCorner } from './maidenhead';

describe('gridToLatLon', () => {
  it('gives the centre of a four-character square', () => {
    expect(gridToLatLon('JN45')).toEqual({ lon: 9, lat: 45.5 });
    expect(gridToLatLon('FN31')).toEqual({ lon: -73, lat: 41.5 });
    expect(gridToLatLon('AA00')).toEqual({ lon: -179, lat: -89.5 });
    expect(gridToLatLon('RR99')).toEqual({ lon: 179, lat: 89.5 });
  });

  it('is more precise with six characters and ignores the case', () => {
    const p = gridToLatLon('jn45ob')!;
    expect(p.lon).toBeCloseTo(8 + (14 * 2) / 24 + 1 / 24, 5);
    expect(p.lat).toBeCloseTo(45 + 1 / 24 + 1 / 48, 5);
  });

  it('refuses what is not a locator', () => {
    for (const bad of ['', 'JN', 'JN4', 'ZZ45', 'JN4A', '1234', 'JN45YY', 'CQ', 'RR7']) expect(gridToLatLon(bad)).toBeNull();
    expect(isGrid('IO91')).toBe(true);
    expect(isGrid(' io91wm ')).toBe(true);
    expect(isGrid('CQ')).toBe(false);
  });
});

describe('squareCorner', () => {
  it('is the lower left corner of the square, from the first four characters', () => {
    expect(squareCorner('JN45')).toEqual({ lon: 8, lat: 45 });
    expect(squareCorner('jn45te')).toEqual({ lon: 8, lat: 45 });
    expect(squareCorner('JN')).toBeNull();
  });
});
