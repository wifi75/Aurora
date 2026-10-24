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
