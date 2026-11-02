import { describe, expect, it } from 'vitest';
import type { Topology } from 'topojson-specification';
import atlas from 'world-atlas/countries-10m.json';
import { shapeAround } from './CountryShape';

const topo = atlas as unknown as Topology;

describe('shapeAround', () => {
  it('finds the country that contains the point (Gambia)', () => {
    const rings = shapeAround(topo, 13.45, -15.6);
    expect(rings.length).toBeGreaterThan(0);
    const xs = rings.flat().map((p) => p[0]);
    expect(Math.max(...xs) - Math.min(...xs)).toBeLessThan(6);
  });

  it('finds small islands near the point when none contains it (Ogasawara)', () => {
    const rings = shapeAround(topo, 27.1, 142.2);
    expect(rings.length).toBeGreaterThan(0);
  });
});
