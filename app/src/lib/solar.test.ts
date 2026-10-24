import { describe, expect, it } from 'vitest';
import { aLevel, arcPoint, conditionLevel, gaugeArc, kLevel, sfiLevel } from './solar';

describe('solar levels', () => {
  it('judges the solar flux: high bands open above 100, closed under 70', () => {
    expect([sfiLevel(150), sfiLevel(100), sfiLevel(99), sfiLevel(70), sfiLevel(69), sfiLevel(null)]).toEqual(['good', 'good', 'fair', 'fair', 'poor', 'none']);
  });

  it('judges the A index: low is calm', () => {
    expect([aLevel(3), aLevel(7), aLevel(8), aLevel(20), aLevel(21), aLevel(null)]).toEqual(['good', 'good', 'fair', 'fair', 'poor', 'none']);
  });

  it('judges the K index: 5 and over is a storm', () => {
    expect([kLevel(0), kLevel(2), kLevel(3), kLevel(4), kLevel(5), kLevel(9), kLevel(null)]).toEqual(['good', 'good', 'fair', 'fair', 'poor', 'poor', 'none']);
  });

  it('reads the forecast words of the service in any case and leaves the rest as unknown', () => {
    expect([conditionLevel('Good'), conditionLevel(' fair '), conditionLevel('POOR'), conditionLevel('Band Closed'), conditionLevel('')]).toEqual(['good', 'fair', 'poor', 'none', 'none']);
  });
});

describe('gauge geometry', () => {
  it('puts the ends of the dial on the left and on the right', () => {
    expect(arcPoint(0).x).toBeCloseTo(20, 5);
    expect(arcPoint(0).y).toBeCloseTo(100, 5);
    expect(arcPoint(1).x).toBeCloseTo(180, 5);
    expect(arcPoint(1).y).toBeCloseTo(100, 5);
    const top = arcPoint(0.5);
    expect(top.x).toBeCloseTo(100, 5);
    expect(top.y).toBeCloseTo(20, 5);
  });

  it('keeps the value inside the dial and survives a bad number', () => {
    expect(arcPoint(5)).toEqual(arcPoint(1));
    expect(arcPoint(-3)).toEqual(arcPoint(0));
    expect(arcPoint(Number.NaN)).toEqual(arcPoint(0));
  });

  it('draws an arc from the left end to the value', () => {
    expect(gaugeArc(0.5)).toBe('M20.00 100.00 A80 80 0 0 1 100.00 20.00');
    expect(gaugeArc(0)).toBe('M20.00 100.00 A80 80 0 0 1 20.00 100.00');
  });
});
