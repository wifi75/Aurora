export interface SunPoint {
  /** Latitudine del punto sotto il Sole (declinazione), gradi. */
  lat: number;
  /** Longitudine del punto sotto il Sole, gradi da -180 a 180. */
  lon: number;
}

const RAD = Math.PI / 180;

/** Dove il Sole e' allo zenit in un istante (formule astronomiche semplici, errore sotto il grado). */
export function sunPosition(date: Date): SunPoint {
  // giorni dal 2000-01-01 12:00 TT, in pratica UTC
  const d = date.getTime() / 86_400_000 + 2_440_587.5 - 2_451_545.0;
  const meanLongitude = (280.46 + 0.9856474 * d) % 360;
  const anomaly = ((357.528 + 0.9856003 * d) % 360) * RAD;
  const eclipticLongitude = (meanLongitude + 1.915 * Math.sin(anomaly) + 0.02 * Math.sin(2 * anomaly)) * RAD;
  const obliquity = (23.439 - 0.0000004 * d) * RAD;
  const declination = Math.asin(Math.sin(obliquity) * Math.sin(eclipticLongitude));
  const rightAscension = Math.atan2(Math.cos(obliquity) * Math.sin(eclipticLongitude), Math.cos(eclipticLongitude));
  const gmst = (280.46061837 + 360.98564736629 * d) % 360;
  let lon = (rightAscension / RAD - gmst) % 360;
  if (lon > 180) lon -= 360;
  if (lon < -180) lon += 360;
  return { lat: declination / RAD, lon };
}

/** Il Sole e' sotto l'orizzonte in quel punto? */
export function isNight(lat: number, lon: number, date: Date): boolean {
  const sun = sunPosition(date);
  const h = (lon - sun.lon) * RAD;
  const sinAlt = Math.sin(lat * RAD) * Math.sin(sun.lat * RAD) + Math.cos(lat * RAD) * Math.cos(sun.lat * RAD) * Math.cos(h);
  return sinAlt < 0;
}

/** Latitudine della linea alba-tramonto alla longitudine data. */
export function terminatorLat(lon: number, sun: SunPoint): number {
  const h = (lon - sun.lon) * RAD;
  // con il Sole sull'equatore la linea passa per i poli: un valore minimo evita la divisione per zero
  const tanDec = Math.tan(Math.abs(sun.lat) < 0.01 ? 0.01 * RAD : sun.lat * RAD);
  return Math.atan(-Math.cos(h) / tanDec) / RAD;
}

/**
 * Il poligono della zona di notte come coppie [lat, lon], su tre giri di mondo (da -540 a 540 gradi) cosi' copre anche le copie
 * della mappa ai lati. La notte sta dalla parte del polo opposto al Sole.
 */
export function nightPolygon(date: Date, stepDeg = 2): Array<[number, number]> {
  const sun = sunPosition(date);
  const ring: Array<[number, number]> = [];
  for (let lon = -540; lon <= 540; lon += stepDeg) ring.push([terminatorLat(lon, sun), lon]);
  const pole = sun.lat >= 0 ? -90 : 90;
  ring.push([pole, 540], [pole, -540]);
  return ring;
}
