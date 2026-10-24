/** La parte di mondo che si vede: x e y in gradi (0-360 da ovest a est, 0-180 da nord a sud), w la larghezza; l'altezza e' w / 2. */
export interface View {
  x: number;
  y: number;
  w: number;
}

export const WORLD_W = 360;
export const WORLD_H = 180;
export const MIN_W = 6;
export const FULL_VIEW: View = { x: 0, y: 0, w: WORLD_W };

/** Longitudine e latitudine in coordinate della mappa (proiezione equirettangolare). */
export function project(lon: number, lat: number): { x: number; y: number } {
  return { x: lon + 180, y: 90 - lat };
}

/** Tiene la vista dentro il mondo e la larghezza tra il minimo e tutto il mondo. */
export function clampView(v: View): View {
  const w = Math.min(WORLD_W, Math.max(MIN_W, v.w));
  const h = w / 2;
  return { w, x: Math.min(WORLD_W - w, Math.max(0, v.x)), y: Math.min(WORLD_H - h, Math.max(0, v.y)) };
}

/** Ingrandisce (factor < 1) o rimpicciolisce (> 1) tenendo ferma la posizione sotto il puntatore, data come frazione (0-1) della vista. */
export function zoomAt(v: View, factor: number, fx: number, fy: number): View {
  const w = Math.min(WORLD_W, Math.max(MIN_W, v.w * factor));
  const h = w / 2;
  const worldX = v.x + fx * v.w;
  const worldY = v.y + fy * (v.w / 2);
  return clampView({ w, x: worldX - fx * w, y: worldY - fy * h });
}

/** Sposta la vista di una frazione della sua larghezza e altezza. */
export function panBy(v: View, dxFrac: number, dyFrac: number): View {
  return clampView({ ...v, x: v.x - dxFrac * v.w, y: v.y - dyFrac * (v.w / 2) });
}
