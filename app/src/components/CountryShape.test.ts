import { describe, expect, it } from 'vitest';
import type { Topology } from 'topojson-specification';
import atlas from 'world-atlas/countries-10m.json';
import landSmall from 'world-atlas/land-110m.json';
import { draw, shapeAround } from './CountryShape';

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

describe('draw', () => {
  const small = landSmall as unknown as Topology;
  it('draws the region around Vanuatu, next to the antimeridian, with the pin inside', () => {
    const d = draw(topo, small, -17.7, 168.3);
    expect(d.country.length).toBeGreaterThan(0);
    expect(d.land.length).toBeGreaterThan(0);
    expect(d.pin[0]).toBeGreaterThan(0);
    expect(d.pin[0]).toBeLessThan(260);
    expect(d.pin[1]).toBeGreaterThan(0);
    expect(d.pin[1]).toBeLessThan(180);
  });

  it('frames Russia wider than Italy', () => {
    const ru = draw(topo, small, 60, 90);
    const it_ = draw(topo, small, 42.5, 12.5);
    expect(ru.frame.w).toBeGreaterThan(it_.frame.w);
  });
});
