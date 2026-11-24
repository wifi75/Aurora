import { browserStorage } from './theme';

const KEY = 'aurora.spot.openPage';

/**
 * «Apri la pagina del modo»: con la scelta accesa un clic su uno spot del DX Cluster porta alla pagina di quel modo (CW, RTTY e PSK nella pagina
 * dei modi testuali, FT8 e FT4 in quella principale) e non solo alla sua frequenza. Spenta di base: come prima, la pagina in cui si e' resta.
 */
export function readOpenModePage(): boolean {
  try {
    return browserStorage()?.getItem(KEY) === '1';
  } catch {
    return false;
  }
}

export function writeOpenModePage(on: boolean): void {
  try {
    browserStorage()?.setItem(KEY, on ? '1' : '0');
  } catch {
    // senza archivio la scelta non si ricorda, ma il cluster funziona lo stesso
  }
}

/** Dove porta uno spot con «Apri la pagina del modo» acceso: `text` (pagina dei modi testuali), `main` (pagina principale), `none` (resta dov'e'). */
export function pageForSpot(textMode: string | null, spotMode: string | null | undefined): 'text' | 'main' | 'none' {
  if (textMode !== null) return 'text';
  const m = (spotMode ?? '').toUpperCase();
  return m === 'FT8' || m === 'FT4' ? 'main' : 'none';
}
