import type { ReactNode } from 'react';
import type { DisplayModel } from '../../lib/radioDisplay';

/** Pezzi di SVG condivisi dai cinque stili: cornice, spie, archi e barre a segmenti. */

export const FONT_DATA = 'Consolas, "JetBrains Mono", ui-monospace, monospace';
export const FONT_UI = '"Segoe UI", "IBM Plex Sans", system-ui, sans-serif';

/** Cio' che serve a ogni stile per disegnarsi: i dati, il testo di "radio non risponde" e l'alternativa testuale. */
export interface StyleProps {
  model: DisplayModel;
  noRadio: string;
  ariaLabel: string;
}

interface FrameProps {
  width: number;
  height: number;
  /** Raggio degli angoli della cornice. */
  radius?: number;
  /** Colore dello schermo (dentro la cornice). */
  screen: string;
  /** Margine fra cornice e schermo. */
  inset?: number;
  ariaLabel: string;
  children: ReactNode;
}

/** Cornice scura dello strumento con lo schermo dentro: in entrambi i temi resta un oggetto scuro, con il bordo che lo stacca dalla pagina. */
export function DisplayFrame({ width, height, radius = 14, screen, inset = 8, ariaLabel, children }: FrameProps) {
  return (
    <svg className="rd-svg" viewBox={`0 0 ${width} ${height}`} role="img" aria-label={ariaLabel} preserveAspectRatio="xMidYMid meet">
      <rect className="rd-bezel" x="1" y="1" width={width - 2} height={height - 2} rx={radius} />
      <rect x={inset} y={inset} width={width - inset * 2} height={height - inset * 2} rx={radius * 0.65} fill={screen} />
      {children}
    </svg>
  );
}

interface ChipProps {
  x: number;
  y: number;
  w: number;
  h?: number;
  text: string;
  bg: string;
  fg: string;
  /** Spia spenta: resta appena visibile. */
  dim?: boolean;
  fontSize?: number;
  rx?: number;
}

export function Chip({ x, y, w, h = 15, text, bg, fg, dim = false, fontSize = 10, rx = 2 }: ChipProps) {
  return (
    <g opacity={dim ? 0.22 : 1} aria-hidden="true">
      <rect x={x} y={y} width={w} height={h} rx={rx} fill={bg} />
      <text x={x + w / 2} y={y + h / 2 + fontSize * 0.36} textAnchor="middle" fontSize={fontSize} fontWeight={600} fill={fg} fontFamily={FONT_UI}>
        {text}
      </text>
    </g>
  );
}

interface TextProps {
  x: number;
  y: number;
  text: string;
  fill: string;
  size?: number;
  weight?: number;
  anchor?: 'start' | 'middle' | 'end';
  opacity?: number;
  spacing?: number;
}

export function Label({ x, y, text, fill, size = 10, weight = 500, anchor = 'start', opacity = 1, spacing }: TextProps) {
  return (
    <text x={x} y={y} fontSize={size} fontWeight={weight} fill={fill} textAnchor={anchor} opacity={opacity} fontFamily={FONT_DATA} letterSpacing={spacing} aria-hidden="true">
      {text}
    </text>
  );
}

/** Punto su una circonferenza (angolo in gradi, 0 = destra, antiorario). */
export function polar(cx: number, cy: number, r: number, deg: number): [number, number] {
  const a = (deg * Math.PI) / 180;
  return [cx + r * Math.cos(a), cy - r * Math.sin(a)];
}

/** Arco da `a0` a `a1` gradi (a0 > a1: in senso orario, da sinistra a destra). */
export function arcPath(cx: number, cy: number, r: number, a0: number, a1: number): string {
  const [x0, y0] = polar(cx, cy, r, a0);
  const [x1, y1] = polar(cx, cy, r, a1);
  return `M${x0.toFixed(1)} ${y0.toFixed(1)} A${r} ${r} 0 0 1 ${x1.toFixed(1)} ${y1.toFixed(1)}`;
}

interface SegBarProps {
  x: number;
  y: number;
  count: number;
  segW: number;
  segH: number;
  step: number;
  /** Quante tacche sono accese. */
  lit: number;
  /** Colore dell'i-esima tacca accesa. */
  color: (i: number) => string;
  /** Colore delle tacche spente. */
  off: string;
  offOpacity?: number;
  rx?: number;
}

/** Barra a tacche rettangolari: le prime `lit` accese, le altre quasi spente. */
export function SegBar({ x, y, count, segW, segH, step, lit, color, off, offOpacity = 1, rx = 0 }: SegBarProps) {
  return (
    <g aria-hidden="true">
      {Array.from({ length: count }, (_, i) => (
        <rect key={i} x={x + i * step} y={y} width={segW} height={segH} rx={rx} fill={i < lit ? color(i) : off} opacity={i < lit ? 1 : offOpacity} />
      ))}
    </g>
  );
}

/** Stato RX/TX per i colori e le spie: in trasmissione RX si spegne. */
export function linkState(model: DisplayModel): { rx: boolean; tx: boolean; busy: boolean } {
  return { rx: !model.transmitting, tx: model.transmitting, busy: model.decoding && !model.transmitting };
}
