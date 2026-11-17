import { describe, expect, it } from 'vitest';
import { FULL_VIEW, ZOOM_MAX, autoView, clampView, fitBox, fitView, footprintPoints, lerpView, panView, projectView, projectViewRaw, unprojectView, viewsClose, wheelZoomFactor, wrapLon, zoomAt, type MapView } from './satView';

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

describe('lerpView logaritmico', () => {
  const a: MapView = { zoom: 1, centerLat: 0, centerLon: 0 };
  const b: MapView = { zoom: 8, centerLat: 40, centerLon: 120 };

  it('a meta lo zoom da 1 a 8 vale circa 2,83 e non 4,5', () => {
    const m = lerpView(a, b, 0.5);
    expect(m.zoom).toBeCloseTo(Math.sqrt(8), 6);
    expect(m.zoom).toBeCloseTo(2.83, 2);
  });

  it('il fattore di zoom per unita di t e costante', () => {
    const z = (t: number) => lerpView(a, b, t).zoom;
    expect(z(0.5) / z(0.25)).toBeCloseTo(z(0.75) / z(0.5), 6);
  });

  it('il centro arriva a destinazione e la longitudine prende il cammino piu corto', () => {
    const e = lerpView(a, b, 1);
    expect(e.centerLat).toBeCloseTo(40);
    expect(e.centerLon).toBeCloseTo(120);
    const w = lerpView({ zoom: 1, centerLat: 0, centerLon: 170 }, { zoom: 8, centerLat: 0, centerLon: -170 }, 0.3);
    // da 170 a -170 si passa per 180 (20 gradi), mai per 0
    expect(Math.abs(wrapLon(w.centerLon - 170))).toBeLessThan(20.001);
    expect(Math.abs(w.centerLon)).toBeGreaterThan(169);
  });

  it('nell ingrandimento il punto di arrivo non esce dallo schermo piu di quanto non fosse all inizio e si avvicina al centro', () => {
    const dest: MapView = { zoom: 8, centerLat: 30, centerLon: 160 };
    const start: MapView = { zoom: 1, centerLat: 0, centerLon: 0 };
    const off = (t: number) => {
      const [x, y] = projectView(dest.centerLat, dest.centerLon, W, H, lerpView(start, dest, t));
      return Math.hypot(x - W / 2, (y - H / 2) * 2);
    };
    let prev = off(0);
    for (let i = 1; i <= 20; i++) {
      const o = off(i / 20);
      expect(o).toBeLessThanOrEqual(prev + 1e-6);
      prev = o;
    }
    expect(off(1)).toBeCloseTo(0, 6);
  });

  it('a zoom uguale il centro e lineare in t', () => {
    const m = lerpView({ zoom: 3, centerLat: 0, centerLon: 0 }, { zoom: 3, centerLat: 20, centerLon: 40 }, 0.25);
    expect(m.centerLat).toBeCloseTo(5);
    expect(m.centerLon).toBeCloseTo(10);
  });
});

describe('zoomAt: punto fermo', () => {
  it('tiene fermo il punto sotto il cursore in vari punti del canvas', () => {
    const v: MapView = { zoom: 2.5, centerLat: 15, centerLon: -30 };
    for (const [x, y] of [[0, 0], [W, H], [450, 225], [100, 400], [800, 30]] as const) {
      for (const f of [1.12, 1 / 1.12, 1.6, 0.7]) {
        const [lat, lon] = unprojectView(x, y, W, H, v);
        const z = zoomAt(v, f, x, y, W, H);
        const maxLat = 90 - 90 / z.zoom;
        const [px, py] = projectView(lat, lon, W, H, z);
        // la longitudine resta sempre ferma; la latitudine, salvo quando il limite del centro la sposta
        expect(px).toBeCloseTo(x, 4);
        if (Math.abs(z.centerLat) < maxLat - 1e-9) expect(py).toBeCloseTo(y, 4);
      }
    }
  });

  it('vicino ai bordi (poli) il clamp tiene la mappa dentro il canvas e la longitudine ferma', () => {
    const v: MapView = { zoom: 2, centerLat: 44, centerLon: 170 };
    // cursore in alto: zoomando indietro il centro vorrebbe salire oltre il limite
    const [lat, lon] = unprojectView(300, 5, W, H, v);
    const z = zoomAt(v, 0.6, 300, 5, W, H);
    expect(z.zoom).toBeCloseTo(1.2, 6);
    expect(z.centerLat).toBeLessThanOrEqual(90 - 90 / z.zoom + 1e-9);
    expect(projectView(lat, lon, W, H, z)[0]).toBeCloseTo(300, 4);
    // al limite dello zoom (8) il fattore non cambia nulla ma la vista resta valida
    const m = zoomAt({ zoom: 8, centerLat: 80, centerLon: 179 }, 2, 450, 10, W, H);
    expect(m.zoom).toBe(ZOOM_MAX);
    expect(m.centerLat).toBeLessThanOrEqual(90 - 90 / 8 + 1e-9);
    expect(m.centerLon).toBeGreaterThanOrEqual(-180);
    expect(m.centerLon).toBeLessThan(180);
  });

  it('attraversa l antimeridiano senza saltare', () => {
    const v: MapView = { zoom: 4, centerLat: 0, centerLon: 179 };
    const [lat, lon] = unprojectView(700, 200, W, H, v);
    const z = zoomAt(v, 1.5, 700, 200, W, H);
    expect(projectView(lat, lon, W, H, z)[0]).toBeCloseTo(700, 4);
  });
});

describe('wheelZoomFactor', () => {
  it('una tacca standard (100) cambia lo zoom di circa il 12%', () => {
    expect(wheelZoomFactor(-100)).toBeCloseTo(1.12, 6);
    expect(wheelZoomFactor(100)).toBeCloseTo(1 / 1.12, 6);
  });

  it('e continuo e reversibile (touchpad con deltaY piccoli)', () => {
    let f = 1;
    for (let i = 0; i < 10; i++) f *= wheelZoomFactor(-10);
    expect(f).toBeCloseTo(wheelZoomFactor(-100), 9);
    expect(wheelZoomFactor(-7) * wheelZoomFactor(7)).toBeCloseTo(1, 12);
    expect(wheelZoomFactor(-3)).toBeGreaterThan(1);
    expect(wheelZoomFactor(-3)).toBeLessThan(1.01);
  });

  it('righe e pagine si riportano ai pixel: 3 righe = una tacca', () => {
    expect(wheelZoomFactor(-3, 1)).toBeCloseTo(wheelZoomFactor(-100, 0), 6);
    expect(wheelZoomFactor(-1, 2)).toBeGreaterThan(wheelZoomFactor(-1, 1));
  });

  it('zero, NaN e valori enormi', () => {
    expect(wheelZoomFactor(0)).toBe(1);
    expect(wheelZoomFactor(Number.NaN)).toBe(1);
    // un solo evento vale al massimo circa 3 tacche
    expect(wheelZoomFactor(-100000)).toBeCloseTo(Math.pow(1.12, 3), 6);
  });
});
