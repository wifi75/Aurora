import { describe, expect, it } from 'vitest';
import { isNight, nightPolygon, sunPosition, terminatorLat } from './terminator';

describe('sunPosition', () => {
  it('has the Sun over the tropic of Cancer at the June solstice, near the Greenwich meridian at noon UTC', () => {
    const s = sunPosition(new Date('2026-06-21T12:00:00Z'));
    expect(s.lat).toBeGreaterThan(23.2);
    expect(s.lat).toBeLessThan(23.5);
    expect(Math.abs(s.lon)).toBeLessThan(3);
  });

  it('has the Sun near the equator at the equinox and over the tropic of Capricorn in December', () => {
    expect(Math.abs(sunPosition(new Date('2026-03-20T15:00:00Z')).lat)).toBeLessThan(0.6);
    expect(sunPosition(new Date('2026-12-21T12:00:00Z')).lat).toBeLessThan(-23.2);
  });

  it('moves west by fifteen degrees every hour', () => {
    const a = sunPosition(new Date('2026-10-24T10:00:00Z')).lon;
    const b = sunPosition(new Date('2026-10-24T11:00:00Z')).lon;
    expect(((a - b + 540) % 360) - 180).toBeCloseTo(15, 0);
  });
});

describe('isNight', () => {
  it('is day under the Sun and night on the opposite side', () => {
    const t = new Date('2026-06-21T12:00:00Z');
    const sun = sunPosition(t);
    expect(isNight(sun.lat, sun.lon, t)).toBe(false);
    expect(isNight(-sun.lat, sun.lon + 180, t)).toBe(true);
  });

  it('has a polar night in the far south in June and a midnight Sun in the far north', () => {
    const t = new Date('2026-06-21T00:00:00Z');
    expect(isNight(-80, 10, t)).toBe(true);
    expect(isNight(80, 10, t)).toBe(false);
  });
});

describe('nightPolygon', () => {
  it('covers three turns of the world and closes on the pole away from the Sun', () => {
    const june = nightPolygon(new Date('2026-06-21T12:00:00Z'));
    expect(june[0]?.[1]).toBe(-540);
    expect(june.at(-3)?.[1]).toBe(540);
    expect(june.at(-1)).toEqual([-90, -540]);
    expect(nightPolygon(new Date('2026-12-21T12:00:00Z')).at(-1)).toEqual([90, -540]);
  });

  it('draws the boundary through the places where the Sun is on the horizon', () => {
    const t = new Date('2026-10-24T12:00:00Z');
    const sun = sunPosition(t);
    for (const lon of [-120, -30, 40, 150]) {
      const lat = terminatorLat(lon, sun);
      expect(Math.abs(lat)).toBeLessThanOrEqual(90);
      // appena sul lato notte della linea e' notte, appena sul lato giorno e' giorno
      const nightSide = sun.lat >= 0 ? lat - 1 : lat + 1;
      const daySide = sun.lat >= 0 ? lat + 1 : lat - 1;
      expect(isNight(nightSide, lon, t)).toBe(true);
      expect(isNight(daySide, lon, t)).toBe(false);
    }
  });

  it('does not divide by zero when the Sun is on the equator', () => {
    const lat = terminatorLat(0, { lat: 0, lon: 0 });
    expect(Number.isFinite(lat)).toBe(true);
  });
});
