import { bearingDeg, gridCenter } from './geo';

/** L'azimut riportato in 0..360 gradi (360 diventa 0). */
export function normalizeAz(az: number): number {
  const a = ((az % 360) + 360) % 360;
  return a >= 360 ? 0 : a;
}

/** Percorso corto: la direzione dal mio locatore a quello del DX (gradi interi, 0 = nord); null se uno dei due non e' valido. */
export function shortPath(myGrid: string | null | undefined, dxGrid: string | null | undefined): number | null {
  const from = myGrid ? gridCenter(myGrid) : null;
  const to = dxGrid ? gridCenter(dxGrid) : null;
  if (!from || !to) return null;
  return bearingDeg(from, to);
}

/** Percorso lungo: dalla parte opposta del globo, corto + 180. */
export function longPath(short: number): number {
  return normalizeAz(short + 180);
}

/** Un passo dei pulsanti «-2°», «+10°»...: sempre dentro 0..360, anche passando dal nord. */
export function nudge(az: number, step: number): number {
  return normalizeAz(Math.round(az) + step);
}

/** La distanza angolare piu' corta tra due azimut (0..180). */
export function angularDistance(a: number, b: number): number {
  const d = normalizeAz(a - b);
  return Math.min(d, 360 - d);
}

/**
 * L'angolo di rotazione da disegnare per arrivare a `az` partendo da quello mostrato prima (`previous`, anche fuori da 0..360): sceglie
 * il giro piu' corto, cosi' l'ago che passa dal nord (359 -> 1) gira di 2 gradi e non di 358.
 */
export function unwrapAngle(previous: number, az: number): number {
  const delta = ((((az - previous) % 360) + 540) % 360) - 180;
  return previous + delta;
}

/** L'azimut di un punto della rosa: (x, y) rispetto al centro, con y verso il basso come sullo schermo. */
export function azimuthOfPoint(x: number, y: number): number {
  const deg = (Math.atan2(x, -y) * 180) / Math.PI;
  return Math.round(normalizeAz(deg));
}

/** Il nome del punto cardinale piu' vicino (sedici settori: N, NNE, NE...). */
export function compassPoint(az: number): string {
  const names = ['N', 'NNE', 'NE', 'ENE', 'E', 'ESE', 'SE', 'SSE', 'S', 'SSW', 'SW', 'WSW', 'W', 'WNW', 'NW', 'NNW'];
  return names[Math.round(normalizeAz(az) / 22.5) % 16] ?? 'N';
}
