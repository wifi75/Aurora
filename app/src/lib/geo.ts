export interface Point {
  lat: number;
  lon: number;
}

const EARTH_KM = 6371;

/** Centro del locatore Maidenhead (4 o 6 caratteri); `null` se non e' un locatore valido. */
export function gridCenter(grid: string): Point | null {
  const g = grid.trim().toUpperCase();
  if (!/^[A-R]{2}[0-9]{2}([A-X]{2})?$/.test(g)) return null;
  const a = (i: number) => g.charCodeAt(i) - 65;
  const d = (i: number) => g.charCodeAt(i) - 48;
  let lon = a(0) * 20 + d(2) * 2 - 180;
  let lat = a(1) * 10 + d(3) - 90;
  if (g.length === 6) {
    lon += a(4) / 12 + 1 / 24;
    lat += a(5) / 24 + 1 / 48;
  } else {
    lon += 1;
    lat += 0.5;
  }
  return { lat, lon };
}

/** Distanza sulla sfera, in chilometri interi. */
export function distanceKm(from: Point, to: Point): number {
  const rad = (deg: number) => (deg * Math.PI) / 180;
  const dLat = rad(to.lat - from.lat);
  const dLon = rad(to.lon - from.lon);
  const h = Math.sin(dLat / 2) ** 2 + Math.cos(rad(from.lat)) * Math.cos(rad(to.lat)) * Math.sin(dLon / 2) ** 2;
  return Math.round(2 * EARTH_KM * Math.asin(Math.min(1, Math.sqrt(h))));
}

/** Il locatore in fondo a un messaggio FT8 (non "RR73", non un rapporto). */
export function gridOfMessage(text: string): string | null {
  const last = text.trim().split(/\s+/).at(-1)?.toUpperCase() ?? '';
  return last !== 'RR73' && /^[A-R]{2}[0-9]{2}$/.test(last) ? last : null;
}

/** Direzione iniziale da un punto a un altro, in gradi da 0 a 359 (0 = nord, 90 = est). */
export function bearingDeg(from: Point, to: Point): number {
  const rad = (deg: number) => (deg * Math.PI) / 180;
  const dLon = rad(to.lon - from.lon);
  const y = Math.sin(dLon) * Math.cos(rad(to.lat));
  const x = Math.cos(rad(from.lat)) * Math.sin(rad(to.lat)) - Math.sin(rad(from.lat)) * Math.cos(rad(to.lat)) * Math.cos(dLon);
  return Math.round(((Math.atan2(y, x) * 180) / Math.PI + 360) % 360) % 360;
}

/**
 * Il percorso piu' breve sulla sfera tra due punti, in `steps` tratti. Le longitudini si tengono continue (possono uscire da
 * -180..180) cosi' una linea che attraversa la linea del cambio data non salta da un lato all'altro della mappa.
 */
export function greatCircle(from: Point, to: Point, steps = 48): Point[] {
  const rad = (deg: number) => (deg * Math.PI) / 180;
  const deg = (r: number) => (r * 180) / Math.PI;
  const [lat1, lon1, lat2, lon2] = [rad(from.lat), rad(from.lon), rad(to.lat), rad(to.lon)];
  const d = 2 * Math.asin(Math.sqrt(Math.sin((lat2 - lat1) / 2) ** 2 + Math.cos(lat1) * Math.cos(lat2) * Math.sin((lon2 - lon1) / 2) ** 2));
  if (d < 1e-9) return [from, to];
  const points: Point[] = [];
  let previous = from.lon;
  for (let i = 0; i <= steps; i++) {
    const f = i / steps;
    const a = Math.sin((1 - f) * d) / Math.sin(d);
    const b = Math.sin(f * d) / Math.sin(d);
    const x = a * Math.cos(lat1) * Math.cos(lon1) + b * Math.cos(lat2) * Math.cos(lon2);
    const y = a * Math.cos(lat1) * Math.sin(lon1) + b * Math.cos(lat2) * Math.sin(lon2);
    const z = a * Math.sin(lat1) + b * Math.sin(lat2);
    let lon = deg(Math.atan2(y, x));
    // porta la longitudine vicina alla precedente: nessun salto di 360 gradi
    while (lon - previous > 180) lon -= 360;
    while (lon - previous < -180) lon += 360;
    previous = lon;
    points.push({ lat: deg(Math.atan2(z, Math.sqrt(x * x + y * y))), lon });
  }
  return points;
}

/** Il locatore Maidenhead di 6 caratteri di un punto («JN45te»); longitudini oltre il giro di mondo si riportano in -180..180. */
export function latLonToGrid(lat: number, lon: number): string {
  const L = (n: number) => String.fromCharCode(65 + n);
  const wrapped = ((((lon + 180) % 360) + 360) % 360) as number;
  const la = Math.min(179.999999, Math.max(0, lat + 90));
  const field = [Math.floor(wrapped / 20), Math.floor(la / 10)];
  const square = [Math.floor((wrapped % 20) / 2), Math.floor(la % 10)];
  const sub = [Math.floor(((wrapped % 2) * 60) / 5), Math.floor(((la % 1) * 60) / 2.5)];
  return `${L(field[0]!)}${L(field[1]!)}${square[0]}${square[1]}${L(sub[0]!).toLowerCase()}${L(sub[1]!).toLowerCase()}`;
}
