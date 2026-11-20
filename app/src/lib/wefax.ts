import { base64ToBytes } from './sstv';

export { base64ToBytes };

/** Gradini di zoom del visore (1 = pixel reali). */
export const ZOOM_STEPS: readonly number[] = [0.1, 0.15, 0.2, 0.25, 0.33, 0.5, 0.67, 0.75, 1, 1.5, 2, 3, 4];

/** Scala che porta l'immagine alla larghezza disponibile: mai oltre 2x (le cartine ingrandite troppo sfocano), mai sotto 5%. */
export function fitScale(availableWidth: number, imageWidth: number): number {
  if (!(availableWidth > 0) || !(imageWidth > 0)) return 1;
  return Math.min(2, Math.max(0.05, availableWidth / imageWidth));
}

/** Il gradino di zoom successivo (dir 1) o precedente (dir -1) a quello di adesso; agli estremi resta dov'e'. */
export function stepZoom(current: number, dir: 1 | -1): number {
  const eps = 1e-6;
  if (dir > 0) return ZOOM_STEPS.find((z) => z > current + eps) ?? ZOOM_STEPS[ZOOM_STEPS.length - 1]!;
  for (let i = ZOOM_STEPS.length - 1; i >= 0; i--) {
    const z = ZOOM_STEPS[i]!;
    if (z < current - eps) return z;
  }
  return ZOOM_STEPS[0]!;
}

/** Il pixel in alto da cui scorrere perche' l'ultima riga attiva resti in fondo al riquadro (mai negativo). */
export function followScrollTop(rows: number, scale: number, viewportHeight: number): number {
  return Math.max(0, Math.round(rows * scale - viewportHeight + 8));
}

/** Copia righe grigie (un byte per pixel) in RGBA (alfa 255) dentro `out`, a partire dalla riga `firstRow` di un'immagine larga `width`. */
export function grayIntoRgba(gray: Uint8Array, out: Uint8ClampedArray, width: number, firstRow: number): void {
  const start = firstRow * width * 4;
  const n = Math.min(gray.length, Math.max(0, Math.floor((out.length - start) / 4)));
  for (let i = 0; i < n; i++) {
    const v = gray[i]!;
    const o = start + i * 4;
    out[o] = v;
    out[o + 1] = v;
    out[o + 2] = v;
    out[o + 3] = 255;
  }
}

/** Le righe grigie in base64 diventano un blocco RGBA pronto per `ImageData` (larghezza `width`); contano solo le righe intere. */
export function decodeGrayRows(b64: string, width: number): { rgba: Uint8ClampedArray<ArrayBuffer>; rows: number } {
  const gray = base64ToBytes(b64);
  const rows = width > 0 ? Math.floor(gray.length / width) : 0;
  const rgba = new Uint8ClampedArray(new ArrayBuffer(rows * width * 4));
  grayIntoRgba(gray.subarray(0, rows * width), rgba, width, 0);
  return { rgba, rows };
}
