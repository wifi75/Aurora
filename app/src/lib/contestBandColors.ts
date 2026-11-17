/**
 * I colori delle bande nelle schede del contest (punti dei moltiplicatori), quelli dei grafici dei programmi da gara:
 * 160 rosso, 80 verde, 40 blu, 20 giallo, 15 magenta, 10 ciano; le altre bande hanno un colore loro, coerente.
 */
const COLORS: Record<string, string> = {
  '160m': '#e53935',
  '80m': '#43a047',
  '60m': '#8d6e63',
  '40m': '#1e88e5',
  '30m': '#9575cd',
  '20m': '#fdd835',
  '17m': '#ff9800',
  '15m': '#d81b8a',
  '12m': '#8bc34a',
  '10m': '#00bcd4',
  '6m': '#ff7043',
  '2m': '#7e57c2',
  '70cm': '#26a69a',
};

/** Il colore della banda (`20m`), grigio se non e' una banda nota. */
export function contestBandColor(band: string): string {
  return COLORS[band.trim().toLowerCase()] ?? '#90a4ae';
}

/** Le bande con il loro colore, dalla piu' bassa (per le legende). */
export const CONTEST_BAND_LEGEND: ReadonlyArray<readonly [string, string]> = Object.entries(COLORS);

/** Ordina le bande dalla piu' bassa (`160m`, `80m`... `10m`, `6m`, `2m`). */
export function sortBands(bands: readonly string[]): string[] {
  const order = Object.keys(COLORS);
  const rank = (b: string) => {
    const i = order.indexOf(b);
    return i < 0 ? order.length : i;
  };
  return [...bands].sort((a, b) => rank(a) - rank(b) || a.localeCompare(b));
}
