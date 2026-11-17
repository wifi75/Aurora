/*
 * Vista (zoom e centro) della mappa dei satelliti: solo logica pura, senza disegno.
 * La mappa e' equirettangolare e il canvas e' largo il doppio dell'altezza (w x w/2): con zoom z si vedono 360/z gradi
 * di longitudine e 180/z di latitudine, con lo stesso numero di pixel per grado nei due assi.
 */
import { footprintPolygon, type LatLon } from './satGeo';

/** Come si muove la mappa da sola quando un satellite e' in vista (nel riquadro degli avvisi). */
export type ZoomMode = 'off' | 'area' | 'both' | 'follow';
export const ZOOM_MODES: readonly ZoomMode[] = ['off', 'area', 'both', 'follow'];
export const DEFAULT_ZOOM_MODE: ZoomMode = 'both';

export const ZOOM_MIN = 1;
export const ZOOM_MAX = 8;
/** Lo zoom fisso del modo «Segui il satellite». */
export const FOLLOW_ZOOM = 3;

export interface MapView {
  /** Da 1 (mondo intero) a 8. */
  zoom: number;
  centerLat: number;
  centerLon: number;
}

export const FULL_VIEW: MapView = { zoom: 1, centerLat: 0, centerLon: 0 };

/** Una longitudine riportata fra -180 (compreso) e 180 (escluso). */
export function wrapLon(lon: number): number {
  return ((((lon + 180) % 360) + 360) % 360) - 180;
}

const clampNum = (v: number, lo: number, hi: number): number => Math.max(lo, Math.min(hi, v));

/** La vista dentro i limiti: zoom da 1 a 8, latitudine del centro tale che la mappa non esca dal canvas, longitudine normalizzata. */
export function clampView(v: MapView, maxZoom = ZOOM_MAX): MapView {
  const zoom = Number.isFinite(v.zoom) ? clampNum(v.zoom, ZOOM_MIN, maxZoom) : ZOOM_MIN;
  const maxLat = 90 - 90 / zoom;
  const lat = Number.isFinite(v.centerLat) ? clampNum(v.centerLat, -maxLat, maxLat) : 0;
  const lon = Number.isFinite(v.centerLon) ? wrapLon(v.centerLon) : 0;
  return { zoom, centerLat: lat, centerLon: lon };
}

/** Pixel per grado di longitudine e di latitudine con questa vista. */
function scales(w: number, h: number, zoom: number): { kx: number; ky: number } {
  return { kx: (w * zoom) / 360, ky: (h * zoom) / 180 };
}

/** Un punto nel canvas w x h con la vista data, nella copia della mappa piu' vicina al centro (la mappa gira senza bordi). */
export function projectView(lat: number, lon: number, w: number, h: number, view: MapView): [number, number] {
  const { kx, ky } = scales(w, h, view.zoom);
  return [w / 2 + wrapLon(lon - view.centerLon) * kx, h / 2 - (lat - view.centerLat) * ky];
}

/** Come `projectView` ma senza riportare la longitudine: serve alle linee con le longitudini «srotolate» (nessun salto di 360 gradi). */
export function projectViewRaw(lat: number, lon: number, w: number, h: number, view: MapView): [number, number] {
  const { kx, ky } = scales(w, h, view.zoom);
  return [w / 2 + (lon - view.centerLon) * kx, h / 2 - (lat - view.centerLat) * ky];
}

/** L'inverso di `projectView`: il punto (latitudine, longitudine) sotto il pixel (x, y). */
export function unprojectView(x: number, y: number, w: number, h: number, view: MapView): [number, number] {
  const { kx, ky } = scales(w, h, view.zoom);
  return [clampNum(view.centerLat - (y - h / 2) / ky, -90, 90), wrapLon(view.centerLon + (x - w / 2) / kx)];
}

/** Cambia lo zoom di un fattore tenendo fermo il punto della mappa sotto il pixel (x, y) (zoom con la rotella o il pizzico). */
export function zoomAt(view: MapView, factor: number, x: number, y: number, w: number, h: number): MapView {
  const [lat, lon] = unprojectView(x, y, w, h, view);
  const zoom = clampNum(view.zoom * factor, ZOOM_MIN, ZOOM_MAX);
  const { kx, ky } = scales(w, h, zoom);
  return clampView({ zoom, centerLat: lat + (y - h / 2) / ky, centerLon: lon - (x - w / 2) / kx });
}

/** Sposta la vista di (dx, dy) pixel come se si trascinasse la mappa: positivo = la mappa va a destra / in basso. */
export function panView(view: MapView, dx: number, dy: number, w: number, h: number): MapView {
  const { kx, ky } = scales(w, h, view.zoom);
  return clampView({ zoom: view.zoom, centerLat: view.centerLat + dy / ky, centerLon: view.centerLon - dx / kx });
}

/** Interpolazione fra due viste: zoom geometrico (la velocita' di ingrandimento resta uniforme), latitudine lineare, longitudine sul cammino piu' corto. */
export function lerpView(a: MapView, b: MapView, t: number): MapView {
  const zoom = Math.exp(Math.log(a.zoom) + (Math.log(b.zoom) - Math.log(a.zoom)) * t);
  const dLon = wrapLon(b.centerLon - a.centerLon);
  return { zoom, centerLat: a.centerLat + (b.centerLat - a.centerLat) * t, centerLon: wrapLon(a.centerLon + dLon * t) };
}

/** Due viste cosi' vicine da essere indistinguibili sullo schermo. */
export function viewsClose(a: MapView, b: MapView): boolean {
  return Math.abs(a.zoom - b.zoom) < 0.004 * a.zoom && Math.abs(a.centerLat - b.centerLat) < 0.02 / a.zoom && Math.abs(wrapLon(a.centerLon - b.centerLon)) < 0.02 / a.zoom;
}

export interface FitOptions {
  /** Margine su ogni lato, in frazione dell'estensione dei punti (0,3 = il 30% in piu' per parte). */
  margin?: number;
  /** Estensione minima (gradi) considerata anche per punti coincidenti, per non ingrandire all'infinito. */
  minSpanDeg?: number;
  maxZoom?: number;
}

/** Zoom e centro che inquadrano tutti i punti (latitudine, longitudine) nel canvas 2:1; le longitudini si prendono dal lato corto, anche a cavallo dell'antimeridiano. */
export function fitView(points: readonly LatLon[], opts: FitOptions = {}): MapView {
  if (points.length === 0) return { ...FULL_VIEW };

  let minLat = Infinity;
  let maxLat = -Infinity;
  for (const p of points) {
    minLat = Math.min(minLat, p[0]);
    maxLat = Math.max(maxLat, p[0]);
  }

  // il lato corto in longitudine: si toglie l'intervallo vuoto piu' largo fra le longitudini sul cerchio
  const lons = points.map((p) => wrapLon(p[1])).sort((x, y) => x - y);
  const n = lons.length;
  let start = lons[0] ?? 0;
  let end = lons[n - 1] ?? 0;
  let widest = start + 360 - end;
  for (let i = 0; i < n - 1; i++) {
    const gap = (lons[i + 1] ?? 0) - (lons[i] ?? 0);
    if (gap > widest) {
      widest = gap;
      start = lons[i + 1] ?? 0;
      end = (lons[i] ?? 0) + 360;
    }
  }
  return fitBox({ south: minLat, north: maxLat, west: start, spanLon: end - start }, opts);
}

/** Un riquadro geografico: latitudini da `south` a `north`, longitudine da `west` verso est per `spanLon` gradi (puo' attraversare l'antimeridiano). */
export interface GeoBox {
  south: number;
  north: number;
  west: number;
  spanLon: number;
}

/** Zoom e centro che inquadrano un riquadro (con il margine e l'estensione minima di `FitOptions`). */
export function fitBox(box: GeoBox, opts: FitOptions = {}): MapView {
  const margin = opts.margin ?? 0.3;
  const minSpan = opts.minSpanDeg ?? 4;
  const maxZoom = opts.maxZoom ?? ZOOM_MAX;
  const spanLat = box.north - box.south;
  const k = 1 + 2 * margin;
  const zoom = Math.min(360 / (Math.max(box.spanLon, minSpan) * k), 180 / (Math.max(spanLat, minSpan / 2) * k));
  return clampView({ zoom, centerLat: (box.south + box.north) / 2, centerLon: box.west + box.spanLon / 2 }, maxZoom);
}

/** Il bordo del cerchio di visibilita' (impronta) del satellite come punti. */
export function footprintPoints(lat: number, lon: number, footprintKm: number): LatLon[] {
  return footprintPolygon(lat, lon, footprintKm, 36);
}

export interface SatSpot {
  lat: number;
  lon: number;
  footprintKm: number;
}

/** La vista da prendere quando un satellite e' in vista, secondo il modo scelto; `null` se lo zoom automatico e' spento. */
export function autoView(mode: ZoomMode, sat: SatSpot, observer: { lat: number; lon: number } | null): MapView | null {
  if (mode === 'off') return null;
  if (mode === 'follow') return clampView({ zoom: FOLLOW_ZOOM, centerLat: sat.lat, centerLon: sat.lon });
  if (mode === 'both' && observer) {
    return fitView(
      [
        [observer.lat, observer.lon],
        [sat.lat, sat.lon],
      ],
      { margin: 0.35, minSpanDeg: 10, maxZoom: 6 },
    );
  }
  return fitView(footprintPoints(sat.lat, sat.lon, sat.footprintKm), { margin: 0.08 });
}
