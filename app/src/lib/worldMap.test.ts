import { describe, expect, it } from 'vitest';
import { worldPaths } from './worldMap';

describe('worldPaths', () => {
  it('draws the land and the borders inside the 360 x 180 map', () => {
    const { land, borders } = worldPaths();
    expect(land.length).toBeGreaterThan(10_000);
    expect(borders.length).toBeGreaterThan(5_000);
    for (const d of [land, borders]) {
      const numbers = d.match(/-?\d+\.\d+/g)!.map(Number);
      expect(Math.min(...numbers)).toBeGreaterThanOrEqual(0);
      expect(Math.max(...numbers)).toBeLessThanOrEqual(360);
    }
    expect(land.startsWith('M')).toBe(true);
  });

  it('is computed once', () => {
    expect(worldPaths()).toBe(worldPaths());
  });
});
