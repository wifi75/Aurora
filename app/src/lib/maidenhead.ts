/** Locatore Maidenhead: 4 caratteri («JN45», un quadrato di 2 x 1 gradi) o 6 («JN45te», 5 x 2,5 primi). */
const GRID = /^[A-R]{2}\d{2}(?:[A-X]{2}(?:\d{2})?)?$/;

/** Un locatore valido. Attenzione: «RR73» ha la forma di un locatore (RR e due cifre), ma nei messaggi e' un saluto: chi legge i messaggi lo scarta a parte. */
export function isGrid(text: string): boolean {
  return GRID.test(text.trim().toUpperCase());
}

export interface Point {
  lat: number;
  lon: number;
}

/** L'angolo in basso a sinistra e il centro di un locatore, o null se non e' valido. */
export function gridToLatLon(grid: string): Point | null {
  const g = grid.trim().toUpperCase();
  if (!GRID.test(g)) return null;
  let lon = (g.charCodeAt(0) - 65) * 20 - 180 + Number(g[2]) * 2;
  let lat = (g.charCodeAt(1) - 65) * 10 - 90 + Number(g[3]);
  if (g.length >= 6) {
    lon += (g.charCodeAt(4) - 65) * (2 / 24) + 1 / 24;
    lat += (g.charCodeAt(5) - 65) * (1 / 24) + 1 / 48;
  } else {
    lon += 1;
    lat += 0.5;
  }
  return { lat, lon };
}

/** Angolo in basso a sinistra (lon, lat) del quadrato di 4 caratteri; i primi quattro caratteri di un locatore piu' lungo contano. */
export function squareCorner(grid: string): Point | null {
  const g = grid.trim().toUpperCase().slice(0, 4);
  if (!GRID.test(g)) return null;
  return { lon: (g.charCodeAt(0) - 65) * 20 - 180 + Number(g[2]) * 2, lat: (g.charCodeAt(1) - 65) * 10 - 90 + Number(g[3]) };
}
