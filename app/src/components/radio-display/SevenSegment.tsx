import type { ReactNode } from 'react';

/** Cifre a sette segmenti disegnate in SVG: i segmenti spenti restano appena visibili, come su un display vero. */

const SEGMENTS: Record<string, string> = {
  '0': 'abcdef',
  '1': 'bc',
  '2': 'abdeg',
  '3': 'abcdg',
  '4': 'bcfg',
  '5': 'acdfg',
  '6': 'acdefg',
  '7': 'abc',
  '8': 'abcdefg',
  '9': 'abcdfg',
  '-': 'g',
};

type Point = readonly [number, number];

interface Metrics {
  x: number;
  y: number;
  w: number;
  h: number;
  /** Spessore del segmento. */
  t: number;
  /** Inclinazione (0 = dritto, 0.1 = corsivo leggero). */
  skew: number;
}

/** Vertici (esagono allungato) di un segmento, gia' inclinati. */
function segmentPoints(name: string, m: Metrics): string {
  const { x, y, w, h, t, skew } = m;
  const mid = y + h / 2;
  const r = t / 2;
  const horizontal = (yy: number): Point[] => [
    [x + r, yy],
    [x + r + t * 0.5, yy - t / 2],
    [x + w - r - t * 0.5, yy - t / 2],
    [x + w - r, yy],
    [x + w - r - t * 0.5, yy + t / 2],
    [x + r + t * 0.5, yy + t / 2],
  ];
  const vertical = (xx: number, y0: number, y1: number): Point[] => [
    [xx, y0 + r],
    [xx + t / 2, y0 + r + t * 0.5],
    [xx + t / 2, y1 - r - t * 0.5],
    [xx, y1 - r],
    [xx - t / 2, y1 - r - t * 0.5],
    [xx - t / 2, y0 + r + t * 0.5],
  ];
  let pts: Point[];
  switch (name) {
    case 'a':
      pts = horizontal(y + t / 2);
      break;
    case 'g':
      pts = horizontal(mid);
      break;
    case 'd':
      pts = horizontal(y + h - t / 2);
      break;
    case 'f':
      pts = vertical(x + t / 2, y, mid);
      break;
    case 'b':
      pts = vertical(x + w - t / 2, y, mid);
      break;
    case 'e':
      pts = vertical(x + t / 2, mid, y + h);
      break;
    default:
      pts = vertical(x + w - t / 2, mid, y + h);
  }
  return pts.map(([px, py]) => `${(px + skew * (y + h - py)).toFixed(1)},${py.toFixed(1)}`).join(' ');
}

export interface DigitStyle {
  w: number;
  h: number;
  t: number;
  gap: number;
  skew: number;
}

/** Larghezza occupata da un testo di cifre (i punti sono piu' stretti delle cifre). */
export function digitsWidth(text: string, s: DigitStyle): number {
  let width = 0;
  for (const c of text) width += c === '.' || c === ',' ? s.t * 1.4 + s.gap * 0.4 : s.w + s.gap;
  return width;
}

interface Props extends DigitStyle {
  text: string;
  x: number;
  y: number;
  /** Colore dei segmenti accesi. */
  on: string;
  /** Alone attorno ai segmenti accesi (colore), facoltativo. */
  glow?: string;
  /** Opacita' dei segmenti spenti. */
  offOpacity?: number;
}

export function SevenSegment({ text, x, y, w, h, t, gap, skew, on, glow, offOpacity = 0.13 }: Props) {
  const nodes: ReactNode[] = [];
  let cx = x;
  [...text].forEach((c, i) => {
    if (c === '.' || c === ',') {
      nodes.push(<circle key={i} cx={cx + t * 0.4 + skew * t} cy={y + h - t * 0.4} r={t * 0.55} fill={on} />);
      cx += t * 1.4 + gap * 0.4;
      return;
    }
    const lit = SEGMENTS[c] ?? '';
    const m: Metrics = { x: cx, y, w, h, t, skew };
    for (const name of 'abcdefg') {
      const isOn = lit.includes(name);
      if (isOn && glow) nodes.push(<polygon key={`${i}${name}g`} points={segmentPoints(name, { ...m, t: t * 1.9 })} fill={glow} opacity={0.16} />);
      nodes.push(<polygon key={`${i}${name}`} points={segmentPoints(name, m)} fill={on} opacity={isOn ? 1 : offOpacity} />);
    }
    cx += w + gap;
  });
  return <g aria-hidden="true">{nodes}</g>;
}
