import { sunPosition, type SunPoint } from './terminator';

const RAD = Math.PI / 180;

/** Dove la Luna e' allo zenit in un istante (formule semplici, errore di un paio di gradi: basta per la mappa). */
export function moonPosition(date: Date): SunPoint {
  const d = date.getTime() / 86_400_000 + 2_440_587.5 - 2_451_545.0;
  const L = 218.316 + 13.176396 * d;
  const M = (134.963 + 13.064993 * d) * RAD;
  const F = (93.272 + 13.22935 * d) * RAD;
  const lon = (L + 6.289 * Math.sin(M)) * RAD;
  const lat = 5.128 * Math.sin(F) * RAD;
  const e = 23.439 * RAD;
  const ra = Math.atan2(Math.sin(lon) * Math.cos(e) - Math.tan(lat) * Math.sin(e), Math.cos(lon));
  const dec = Math.asin(Math.sin(lat) * Math.cos(e) + Math.cos(lat) * Math.sin(e) * Math.sin(lon));
  const gmst = (280.46061837 + 360.98564736629 * d) % 360;
  let sub = (ra / RAD - gmst) % 360;
  if (sub > 180) sub -= 360;
  if (sub < -180) sub += 360;
  return { lat: dec / RAD, lon: sub };
}

/** Fase della Luna da 0 (nuova) a 1 (di nuovo nuova); 0,5 = piena. */
export function moonPhase(date: Date): number {
  const days = date.getTime() / 86_400_000 - 10_962.6; // luna nuova del 6 gennaio 2000 alle 18:14 UTC
  const p = (days / 29.530588853) % 1;
  return p < 0 ? p + 1 : p;
}

/**
 * Il poligono dove il Sole sta sotto `altitude` gradi (0 = notte; -6, -12, -18 = fine dei crepuscoli), su tre giri di mondo come la notte.
 * Per ogni longitudine si parte dal polo al buio e si scende finche' il Sole non sale sopra `altitude`: e' il bordo.
 */
export function shadowPolygon(date: Date, altitude: number, stepDeg = 2): Array<[number, number]> {
  const sun = sunPosition(date);
  const pole = sun.lat >= 0 ? -90 : 90;
  const dir = pole > 0 ? -1 : 1;
  const sinD = Math.sin(sun.lat * RAD);
  const cosD = Math.cos(sun.lat * RAD);
  const limit = Math.sin(altitude * RAD);
  const edge = new Map<number, number>();
  for (let lon = -180; lon <= 180; lon += stepDeg) {
    const cosH = Math.cos((lon - sun.lon) * RAD);
    let lat = pole;
    for (let step = 0; step <= 360; step++) {
      const phi = pole + dir * step * 0.5;
      const alt = Math.sin(phi * RAD) * sinD + Math.cos(phi * RAD) * cosD * cosH;
      if (alt > limit) break;
      lat = phi;
    }
    edge.set(lon, lat);
  }
  const ring: Array<[number, number]> = [];
  for (let lon = -540; lon <= 540; lon += stepDeg) ring.push([edge.get(((((lon + 180) % 360) + 360) % 360) - 180) ?? pole, lon]);
  ring.push([pole, 540], [pole, -540]);
  return ring;
}
