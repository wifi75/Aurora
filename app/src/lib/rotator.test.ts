import { describe, expect, it } from 'vitest';
import { angularDistance, azimuthOfPoint, compassPoint, longPath, normalizeAz, nudge, shortPath, unwrapAngle } from './rotator';

describe('rotore', () => {
  it('riporta gli azimut in 0..360', () => {
    expect(normalizeAz(370)).toBe(10);
    expect(normalizeAz(-2)).toBe(358);
    expect(normalizeAz(360)).toBe(0);
  });

  it('i passi girano attorno al nord', () => {
    expect(nudge(359, 2)).toBe(1);
    expect(nudge(1, -2)).toBe(359);
    expect(nudge(355, 10)).toBe(5);
    expect(nudge(5, -10)).toBe(355);
    expect(nudge(90.4, 2)).toBe(92);
  });

  it('percorso corto e lungo dal locatore', () => {
    // da IO91wm (Londra) verso est e verso ovest
    const east = shortPath('IO91wm', 'KN04');
    expect(east).not.toBeNull();
    expect(east!).toBeGreaterThan(80);
    expect(east!).toBeLessThan(110);
    const west = shortPath('IO91wm', 'FN31');
    expect(west!).toBeGreaterThan(270);
    expect(west!).toBeLessThan(310);
    expect(longPath(east!)).toBe(normalizeAz(east! + 180));
    expect(longPath(300)).toBe(120);
    expect(shortPath('IO91wm', null)).toBeNull();
    expect(shortPath('xx', 'FN31')).toBeNull();
  });

  it('distanza angolare e giro piu’ corto', () => {
    expect(angularDistance(359, 1)).toBe(2);
    expect(angularDistance(10, 190)).toBe(180);
    expect(unwrapAngle(359, 1)).toBe(361);
    expect(unwrapAngle(1, 359)).toBe(-1);
    expect(unwrapAngle(720, 90)).toBe(810);
  });

  it('un clic sulla rosa da’ l’azimut', () => {
    expect(azimuthOfPoint(0, -10)).toBe(0);
    expect(azimuthOfPoint(10, 0)).toBe(90);
    expect(azimuthOfPoint(0, 10)).toBe(180);
    expect(azimuthOfPoint(-10, 0)).toBe(270);
    expect(compassPoint(44)).toBe('NE');
    expect(compassPoint(350)).toBe('N');
  });
});
