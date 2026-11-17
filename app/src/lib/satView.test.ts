import { describe, expect, it } from 'vitest';
import { FULL_VIEW, ZOOM_MAX, autoView, clampView, fitBox, fitView, footprintPoints, lerpView, panView, projectView, projectViewRaw, unprojectView, viewsClose, wrapLon, zoomAt, type MapView } from './satView';

const W = 900;
const H = 450;

/** Tutti i punti dentro il canvas (con una tolleranza)? */
function allInside(points: readonly (readonly [number, number])[], view: MapView, tol = 1): boolean {
  return points.every(([lat, lon]) => {
    const [x, y] = projectView(lat, lon, W, H, view);
    return x >= -tol && x <= W + tol && y >= -tol && y <= H + tol;
  });
}

describe('wrapLon e clampView', () => {
  it('porta le longitudini fra -180 e 180', () => {
    expect(wrapLon(190)).toBeCloseTo(-170);
    expect(wrapLon(-190)).toBeCloseTo(170);
    expect(wrapLon(540)).toBeCloseTo(-180);
    expect(wrapLon(10)).toBeCloseTo(10);
  });

  it('limita lo zoom a 1-8 e la latitudine del centro in modo che la mappa non esca dal canvas', () => {
    expect(clampView({ zoom: 0.2, centerLat: 50, centerLon: 0 })).toEqual({ zoom: 1, centerLat: 0, centerLon: 0 });
    expect(clampView({ zoom: 20, centerLat: 0, centerLon: 0 }).zoom).toBe(ZOOM_MAX);
    // a zoom 2 si vedono 90 gradi di latitudine: il centro arriva al massimo a 45
    expect(clampView({ zoom: 2, centerLat: 80, centerLon: 0 }).centerLat).toBeCloseTo(45);
    expect(clampView({ zoom: 2, centerLat: -80, centerLon: 0 }).centerLat).toBeCloseTo(-45);
    expect(clampView({ zoom: 4, centerLat: 10, centerLon: 200 }).centerLon).toBeCloseTo(-160);
  });

  it('i valori non numerici tornano alla vista intera', () => {
    expect(clampView({ zoom: Number.NaN, centerLat: Number.NaN, centerLon: Number.NaN })).toEqual(FULL_VIEW);
  });
});

describe('fitView', () => {
  it('stazione e satellite vicini: zoom alto', () => {
    const v = fitView([
      [45, 10],
      [48, 14],
    ]);
    expect(v.zoom).toBeGreaterThan(4);
    expect(allInside([[45, 10], [48, 14]], v)).toBe(true);
    expect(v.centerLat).toBeCloseTo(46.5, 1);
    expect(v.centerLon).toBeCloseTo(12, 1);
  });

  it('stazione e satellite lontani: zoom basso', () => {
    const v = fitView([
      [60, -100],
      [-40, 100],
    ]);
    expect(v.zoom).toBeLessThan(1.5);
    expect(allInside([[60, -100], [-40, 100]], v, 2)).toBe(true);
  });

  it('punti agli antipodi: vista intera', () => {
    const v = fitView([
      [0, -170],
      [0, 170],
      [0, 0],
      [0, 90],
      [0, -90],
    ]);
    expect(v.zoom).toBe(1);
  });

  it('a cavallo dell antimeridiano sceglie il lato corto', () => {
    const v = fitView([
      [10, 175],
      [20, -175],
    ]);
    expect(Math.abs(v.centerLon)).toBeGreaterThan(175);
    expect(v.zoom).toBeGreaterThan(4);
    expect(allInside([[10, 175], [20, -175]], v)).toBe(true);
  });

  it('un punto solo non ingrandisce all infinito', () => {
    const v = fitView([[10, 20]], { maxZoom: 6 });
    expect(v.zoom).toBeLessThanOrEqual(6);
    expect(v.centerLat).toBeCloseTo(10);
    expect(v.centerLon).toBeCloseTo(20);
  });

  it('senza punti: vista intera', () => {
    expect(fitView([])).toEqual(FULL_VIEW);
  });

  it('il margine rimpicciolisce lo zoom', () => {
    const pts: [number, number][] = [
      [10, 0],
      [60, 60],
    ];
    expect(fitView(pts, { margin: 0.1 }).zoom).toBeGreaterThan(fitView(pts, { margin: 0.5 }).zoom);
  });

  it('l impronta di un satellite sulla sfera sta tutta nel canvas', () => {
    const pts = footprintPoints(40, 10, 2500);
    const v = fitView(pts, { margin: 0.08 });
    expect(v.zoom).toBeGreaterThan(1);
    expect(allInside(pts, v, 2)).toBe(true);
  });

  it('l impronta a cavallo dell antimeridiano e vicino al polo', () => {
    const across = footprintPoints(0, 180, 2500);
    const v = fitView(across, { margin: 0.08 });
    expect(Math.abs(v.centerLon)).toBeGreaterThan(170);
    expect(allInside(across, v, 2)).toBe(true);
    // un'impronta che comprende il polo copre tutte le longitudini: vista intera
    expect(fitView(footprintPoints(88, 0, 3000)).zoom).toBe(1);
  });
});

describe('autoView', () => {
  const sat = { lat: 45, lon: 12, footprintKm: 2400 };
  const me = { lat: 45.5, lon: 11 };

  it('spento: nessuna vista', () => {
    expect(autoView('off', sat, me)).toBeNull();
  });

  it('segui il satellite: zoom fisso e centro sul satellite', () => {
    const v = autoView('follow', sat, me);
    expect(v?.zoom).toBe(3);
    expect(v?.centerLat).toBeCloseTo(45);
    expect(v?.centerLon).toBeCloseTo(12);
  });

  it('stazione e satellite: li inquadra entrambi', () => {
    const v = autoView('both', { ...sat, lat: 50, lon: 20 }, me);
    expect(v).not.toBeNull();
    if (v) expect(allInside([[me.lat, me.lon], [50, 20]], v)).toBe(true);
  });

  it('area del satellite e stazione e satellite senza stazione: l impronta', () => {
    const a = autoView('area', sat, me);
    const b = autoView('both', sat, null);
    expect(a).not.toBeNull();
    expect(b).toEqual(a);
  });
});

describe('proiezione', () => {
  it('a vista intera coincide con la proiezione equirettangolare', () => {
    const [x, y] = projectView(0, 0, W, H, FULL_VIEW);
    expect(x).toBeCloseTo(W / 2);
    expect(y).toBeCloseTo(H / 2);
    const [x2, y2] = projectView(90, -180, W, H, FULL_VIEW);
    expect(x2).toBeCloseTo(0);
    expect(y2).toBeCloseTo(0);
  });

  it('il centro della vista va al centro del canvas', () => {
    const v: MapView = { zoom: 4, centerLat: 30, centerLon: 70 };
    const [x, y] = projectView(30, 70, W, H, v);
    expect(x).toBeCloseTo(W / 2);
    expect(y).toBeCloseTo(H / 2);
  });

  it('proiezione e inversa sono coerenti', () => {
    const views: MapView[] = [FULL_VIEW, { zoom: 3, centerLat: 20, centerLon: -40 }, { zoom: 8, centerLat: -60, centerLon: 179 }];
    for (const v of views) {
      for (const [lat, lon] of [[10, 5], [-20, 100], [40, -170]] as const) {
        const [x, y] = projectView(lat, lon, W, H, v);
        const [la, lo] = unprojectView(x, y, W, H, v);
        // a zoom alto il punto puo' stare fuori dal canvas ma la coerenza vale lo stesso, salvo il giro di 360
        expect(la).toBeCloseTo(lat, 6);
        expect(wrapLon(lo - lon)).toBeCloseTo(0, 6);
      }
    }
  });

  it('la versione senza giro differisce di multipli della larghezza del mondo', () => {
    const v: MapView = { zoom: 2, centerLat: 0, centerLon: 0 };
    const [xa] = projectView(0, 170, W, H, v);
    const [xb] = projectViewRaw(0, 170 + 360, W, H, v);
    expect(xb - xa).toBeCloseTo(W * v.zoom);
  });

  it('a zoom 2 la larghezza visibile e 180 gradi', () => {
    const v: MapView = { zoom: 2, centerLat: 0, centerLon: 0 };
    expect(projectView(0, 90, W, H, v)[0]).toBeCloseTo(W);
    expect(projectView(0, -90, W, H, v)[0]).toBeCloseTo(0);
    expect(projectView(45, 0, W, H, v)[1]).toBeCloseTo(0);
  });
});

describe('zoomAt e panView', () => {
  it('la rotella tiene fermo il punto sotto il cursore', () => {
    const v: MapView = { zoom: 2, centerLat: 10, centerLon: 20 };
    const [lat, lon] = unprojectView(300, 120, W, H, v);
    const z = zoomAt(v, 1.5, 300, 120, W, H);
    expect(z.zoom).toBeCloseTo(3);
    const [x, y] = projectView(lat, lon, W, H, z);
    expect(x).toBeCloseTo(300, 4);
    expect(y).toBeCloseTo(120, 4);
  });

  it('non va sotto 1 ne sopra 8', () => {
    expect(zoomAt(FULL_VIEW, 0.5, 100, 100, W, H).zoom).toBe(1);
    expect(zoomAt({ zoom: 7, centerLat: 0, centerLon: 0 }, 3, 100, 100, W, H).zoom).toBe(ZOOM_MAX);
  });

  it('il trascinamento sposta anche in verticale solo con lo zoom', () => {
    const full = panView(FULL_VIEW, 90, 90, W, H);
    expect(full.centerLat).toBe(0);
    expect(full.centerLon).toBeCloseTo(-36);
    const z = panView({ zoom: 4, centerLat: 0, centerLon: 0 }, 0, 90, W, H);
    expect(z.centerLat).toBeGreaterThan(0);
    // trascinando la mappa in basso si vede la parte piu' a nord
    expect(z.centerLat).toBeCloseTo(9, 6);
  });

  it('il trascinamento in verticale non esce dalla mappa', () => {
    const v = panView({ zoom: 2, centerLat: 0, centerLon: 0 }, 0, 100000, W, H);
    expect(v.centerLat).toBeCloseTo(45);
  });
});

describe('lerpView', () => {
  it('agli estremi da le viste di partenza e di arrivo', () => {
    const a: MapView = { zoom: 1, centerLat: 0, centerLon: 0 };
    const b: MapView = { zoom: 4, centerLat: 30, centerLon: 60 };
    expect(lerpView(a, b, 0)).toEqual(a);
    const e = lerpView(a, b, 1);
    expect(e.zoom).toBeCloseTo(4);
    expect(e.centerLat).toBeCloseTo(30);
    expect(e.centerLon).toBeCloseTo(60);
  });

  it('lo zoom cresce in modo geometrico', () => {
    expect(lerpView({ zoom: 1, centerLat: 0, centerLon: 0 }, { zoom: 4, centerLat: 0, centerLon: 0 }, 0.5).zoom).toBeCloseTo(2);
  });

  it('la longitudine segue il cammino piu corto attraverso l antimeridiano', () => {
    const a: MapView = { zoom: 2, centerLat: 0, centerLon: 170 };
    const b: MapView = { zoom: 2, centerLat: 0, centerLon: -170 };
    const m = lerpView(a, b, 0.5);
    expect(Math.abs(m.centerLon)).toBeCloseTo(180);
    const q = lerpView(a, b, 0.25);
    expect(q.centerLon).toBeCloseTo(175);
    const r = lerpView(b, a, 0.25);
    expect(r.centerLon).toBeCloseTo(-175);
  });

  it('viewsClose riconosce le viste indistinguibili', () => {
    const a: MapView = { zoom: 3, centerLat: 10, centerLon: 20 };
    expect(viewsClose(a, { ...a, centerLon: 20.001 })).toBe(true);
    expect(viewsClose(a, { ...a, zoom: 3.5 })).toBe(false);
    expect(viewsClose({ zoom: 2, centerLat: 0, centerLon: 179.999 }, { zoom: 2, centerLat: 0, centerLon: -179.999 })).toBe(true);
  });
});

describe('fitBox', () => {
  it("inquadra un riquadro a cavallo dell'antimeridiano dal lato corto", () => {
    const v = fitBox({ south: -20, north: -12, west: 177, spanLon: 6 }, { margin: 0.2, minSpanDeg: 4 });
    expect(Math.abs(wrapLon(v.centerLon - 180))).toBeLessThan(0.01);
    expect(v.centerLat).toBeCloseTo(-16, 5);
    expect(
      allInside(
        [
          [-20, 177],
          [-12, -177],
        ],
        v,
      ),
    ).toBe(true);
  });
  it('un riquadro minuscolo non supera lo zoom massimo e uno enorme resta a vista intera', () => {
    expect(fitBox({ south: 10, north: 10.1, west: 20, spanLon: 0.1 }).zoom).toBe(ZOOM_MAX);
    expect(fitBox({ south: -90, north: -60, west: -180, spanLon: 360 }).zoom).toBe(1);
  });
  it('fitView e fitBox danno la stessa vista', () => {
    const pts: [number, number][] = [
      [40, 10],
      [46, 20],
    ];
    expect(fitView(pts)).toEqual(fitBox({ south: 40, north: 46, west: 10, spanLon: 10 }));
  });
});
