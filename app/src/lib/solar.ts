export type Level = 'good' | 'fair' | 'poor' | 'none';

/** Flusso solare (SFI): sopra 100 le bande alte funzionano, sotto 70 sono chiuse. */
export function sfiLevel(sfi: number | null): Level {
  if (sfi === null) return 'none';
  if (sfi >= 100) return 'good';
  return sfi >= 70 ? 'fair' : 'poor';
}

/** Indice A: piu' e' basso, piu' il campo magnetico e' calmo. */
export function aLevel(a: number | null): Level {
  if (a === null) return 'none';
  if (a < 8) return 'good';
  return a <= 20 ? 'fair' : 'poor';
}

/** Indice K (0-9): da 5 in su c'e' una tempesta geomagnetica. */
export function kLevel(k: number | null): Level {
  if (k === null) return 'none';
  if (k <= 2) return 'good';
  return k <= 4 ? 'fair' : 'poor';
}

/** La condizione calcolata come la scrive hamqsl.com («Good», «Fair», «Poor»), in un livello. */
export function conditionLevel(condition: string): Level {
  switch (condition.trim().toLowerCase()) {
    case 'good':
      return 'good';
    case 'fair':
      return 'fair';
    case 'poor':
      return 'poor';
    default:
      return 'none';
  }
}

/** Il punto sul semicerchio di un quadrante: 0 = a sinistra, 1 = a destra (centro 100,100, raggio 80). */
export function arcPoint(fraction: number, radius = 80, cx = 100, cy = 100): { x: number; y: number } {
  const f = Math.min(1, Math.max(0, Number.isFinite(fraction) ? fraction : 0));
  const angle = Math.PI * (1 - f);
  return { x: cx + radius * Math.cos(angle), y: cy - radius * Math.sin(angle) };
}

/** L'arco colorato di un quadrante, dall'inizio fino alla frazione data. */
export function gaugeArc(fraction: number, radius = 80): string {
  const f = Math.min(1, Math.max(0, Number.isFinite(fraction) ? fraction : 0));
  const start = arcPoint(0, radius);
  const end = arcPoint(f, radius);
  return `M${start.x.toFixed(2)} ${start.y.toFixed(2)} A${radius} ${radius} 0 0 1 ${end.x.toFixed(2)} ${end.y.toFixed(2)}`;
}
