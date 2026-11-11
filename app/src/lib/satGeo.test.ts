import { describe, expect, it } from 'vitest';
import { compass, countdown, destination, footprintPolygon, mmss, nightAlpha, polarPoint, project, splitAtAntimeridian, sunAltitudeDeg } from './satGeo';

describe('project', () => {
  it('mette gli angoli della mappa agli angoli dell\'immagine', () => {
    expect(project(90, -180, 1440, 720)).toEqual([0, 0]);
    expect(project(-90, 180, 1440, 720)).toEqual([1440, 720]);
    expect(project(0, 0, 1440, 720)).toEqual([720, 360]);
  });
});

describe('splitAtAntimeridian', () => {
  it('spezza dove la traccia passa da 179 a -179', () => {
    const segs = splitAtAntimeridian([[10, 170], [11, 178], [12, -178], [13, -170]]);
    expect(segs).toHaveLength(2);
    expect(segs[0]).toEqual([[10, 170], [11, 178]]);
    expect(segs[1]).toEqual([[12, -178], [13, -170]]);
  });

  it('una traccia senza salti resta intera', () => {
    expect(splitAtAntimeridian([[0, 0], [1, 10], [2, 20]])).toHaveLength(1);
    expect(splitAtAntimeridian([])).toEqual([]);
  });
});

describe('destination e footprintPolygon', () => {
  it('un grado di latitudine verso nord sono circa 111 km', () => {
    const [lat, lon] = destination(0, 0, 0, 111.19);
    expect(lat).toBeCloseTo(1, 1);
    expect(lon).toBeCloseTo(0, 3);
  });

  it('l\'impronta e\' un cerchio chiuso a distanza costante', () => {
    const pts = footprintPolygon(45, 11, 2000, 36);
    expect(pts).toHaveLength(37);
    expect(pts[0]?.[0]).toBeCloseTo(pts[36]?.[0] ?? 0, 6);
    // il punto a nord e' circa 18 gradi piu' su
    expect(pts[0]?.[0]).toBeCloseTo(45 + 2000 / 111.19, 0);
  });
});

describe('Sole e notte', () => {
  it('il Sole allo zenit e\' a 90 gradi, dall\'altra parte e\' sotto l\'orizzonte', () => {
    expect(sunAltitudeDeg(20, 30, 20, 30)).toBeCloseTo(90, 5);
    expect(sunAltitudeDeg(-20, -150, 20, 30)).toBeCloseTo(-90, 5);
  });

  it('di giorno non si scurisce, di notte si', () => {
    expect(nightAlpha(30)).toBe(0);
    expect(nightAlpha(-30)).toBeCloseTo(0.62, 5);
    expect(nightAlpha(-9)).toBeCloseTo(0.31, 5);
  });
});

describe('formati', () => {
  it('punti cardinali', () => {
    expect(compass(0)).toBe('N');
    expect(compass(92)).toBe('E');
    expect(compass(225)).toBe('SW');
    expect(compass(359)).toBe('N');
  });

  it('durate e tempi mancanti', () => {
    expect(mmss(605)).toBe('10:05');
    expect(countdown(45)).toBe('45 s');
    expect(countdown(200)).toBe('3 min 20 s');
    expect(countdown(3900)).toBe('1 h 05 min');
  });
});

describe('polarPoint', () => {
  it('lo zenit e\' al centro e l\'orizzonte sul bordo', () => {
    expect(polarPoint(123, 90, 100)).toEqual([0, 0]);
    const [x, y] = polarPoint(0, 0, 100);
    expect(x).toBeCloseTo(0, 6);
    expect(y).toBeCloseTo(-100, 6);
    const [ex, ey] = polarPoint(90, 0, 100);
    expect(ex).toBeCloseTo(100, 6);
    expect(ey).toBeCloseTo(0, 6);
  });
});
