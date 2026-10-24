/** Un colore per banda, distinguibili tra loro anche su una mappa chiara e su una scura. */
const BAND_COLORS: Record<string, string> = {
  '160m': '#8e44ad',
  '80m': '#6c5ce7',
  '60m': '#2980b9',
  '40m': '#00a8e8',
  '30m': '#00b894',
  '20m': '#2ecc71',
  '17m': '#a3cb38',
  '15m': '#f1c40f',
  '12m': '#f39c12',
  '10m': '#e67e22',
  '6m': '#e74c3c',
  '2m': '#ff4d8d',
};

export function bandColor(band: string | null | undefined): string {
  return (band && BAND_COLORS[band]) || '#7f8c8d';
}

/** Le bande con il loro colore, nell'ordine dalla piu' bassa, per le legende. */
export const BAND_LEGEND: ReadonlyArray<readonly [string, string]> = Object.entries(BAND_COLORS);

/** Il colore di una stazione in elenco secondo la sua etichetta; `fallback` se non c'e' una variabile per quell'etichetta. */
export function tagColorName(tag: string | null | undefined, cq: boolean): { variable: string | null; fallback: string } {
  if (tag && tag !== 'cq' && tag !== 'auto') return { variable: `--tag-${tag.replaceAll('_', '-')}`, fallback: '#f2c94c' };
  return { variable: null, fallback: cq ? '#3fd6c8' : '#7aa2ff' };
}
