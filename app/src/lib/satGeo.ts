/** Geometria della mappa dei satelliti: proiezione equirettangolare, tracce che attraversano l'antimeridiano, impronta radio e zona di notte. */

export type LatLon = readonly [number, number];

/** Il nome salvato nelle impostazioni per la mappa inclusa in Aurora (non e' un file della cartella `maps`). */
export const SAT_BUILTIN_MAP = ':aurora';

const RAD = Math.PI / 180;
const EARTH_KM = 6371;

/** Un punto sulla mappa a proiezione equirettangolare (longitudine -180..180 verso destra, latitudine 90..-90 verso il basso). */
export function project(lat: number, lon: number, width: number, height: number): [number, number] {
  return [((lon + 180) / 360) * width, ((90 - lat) / 180) * height];
}

/** Spezza una traccia dove attraversa l'antimeridiano (salto di piu' di 180 gradi di longitudine), per non disegnare la riga attraverso la mappa. */
export function splitAtAntimeridian(points: readonly LatLon[]): LatLon[][] {
  const out: LatLon[][] = [];
  let cur: LatLon[] = [];
  let prev: LatLon | null = null;
  for (const p of points) {
    if (prev && Math.abs(p[1] - prev[1]) > 180) {
      if (cur.length > 0) out.push(cur);
      cur = [];
    }
    cur.push(p);
    prev = p;
  }
  if (cur.length > 0) out.push(cur);
  return out;
}

/** Il punto a `distanceKm` da (lat, lon) in direzione `bearingDeg` sulla sfera. */
export function destination(lat: number, lon: number, bearingDeg: number, distanceKm: number): LatLon {
  const d = distanceKm / EARTH_KM;
  const b = bearingDeg * RAD;
  const p1 = lat * RAD;
  const l1 = lon * RAD;
  const p2 = Math.asin(Math.sin(p1) * Math.cos(d) + Math.cos(p1) * Math.sin(d) * Math.cos(b));
  const l2 = l1 + Math.atan2(Math.sin(b) * Math.sin(d) * Math.cos(p1), Math.cos(d) - Math.sin(p1) * Math.sin(p2));
  return [p2 / RAD, ((((l2 / RAD + 180) % 360) + 360) % 360) - 180];
}

/** La linea dell'impronta radio (il cerchio all'orizzonte del satellite) come punti: va spezzata dove attraversa l'antimeridiano. */
export function footprintPolygon(lat: number, lon: number, radiusKm: number, steps = 90): LatLon[] {
  const pts: LatLon[] = [];
  for (let i = 0; i <= steps; i++) pts.push(destination(lat, lon, (i / steps) * 360, radiusKm));
  return pts;
}

/** L'altezza del Sole (gradi) in un punto, dato il punto in cui il Sole e' allo zenit. */
export function sunAltitudeDeg(lat: number, lon: number, sunLat: number, sunLon: number): number {
  const s = Math.sin(lat * RAD) * Math.sin(sunLat * RAD) + Math.cos(lat * RAD) * Math.cos(sunLat * RAD) * Math.cos((lon - sunLon) * RAD);
  return Math.asin(Math.max(-1, Math.min(1, s))) / RAD;
}

/** Quanto scurire un punto: 0 di giorno, fino a `max` di notte piena, con il crepuscolo in mezzo (da 0 a -18 gradi). */
export function nightAlpha(sunAltitude: number, max = 0.62): number {
  if (sunAltitude >= 0) return 0;
  if (sunAltitude <= -18) return max;
  return (-sunAltitude / 18) * max;
}

/** Un angolo come punto cardinale («N», «NE»...). */
export function compass(azDeg: number): string {
  const names = ['N', 'NE', 'E', 'SE', 'S', 'SW', 'W', 'NW'];
  return names[Math.round((((azDeg % 360) + 360) % 360) / 45) % 8] ?? 'N';
}

/** Durata in «m:ss». */
export function mmss(seconds: number): string {
  const s = Math.max(0, Math.round(seconds));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

/** Tempo mancante: «3 min 20 s», «1 h 05 min». */
export function countdown(seconds: number): string {
  const s = Math.max(0, Math.round(seconds));
  if (s >= 3600) return `${Math.floor(s / 3600)} h ${String(Math.floor((s % 3600) / 60)).padStart(2, '0')} min`;
  if (s >= 60) return `${Math.floor(s / 60)} min ${String(s % 60).padStart(2, '0')} s`;
  return `${s} s`;
}

/** Latitudine o longitudine come «33,80° N». */
export function formatLat(lat: number): string {
  return `${Math.abs(lat).toFixed(2)}° ${lat >= 0 ? 'N' : 'S'}`;
}

export function formatLon(lon: number): string {
  return `${Math.abs(lon).toFixed(2)}° ${lon >= 0 ? 'E' : 'W'}`;
}

/** Frequenza in MHz con sei decimali (per le radio da satellite). */
export function mhz(hz: number): string {
  return (hz / 1e6).toFixed(6);
}

/** Il punto di una traccia polare (azimut ed elevazione) come coordinate sul disco: l'orizzonte e' sul bordo, lo zenit al centro, il nord in alto. */
export function polarPoint(azDeg: number, elDeg: number, radius: number): [number, number] {
  const r = ((90 - Math.max(0, Math.min(90, elDeg))) / 90) * radius;
  const a = azDeg * RAD;
  return [r * Math.sin(a), -r * Math.cos(a)];
}
