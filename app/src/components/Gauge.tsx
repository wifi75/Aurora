import { gaugeArc, type Level } from '../lib/solar';

interface Props {
  label: string;
  /** Il valore e il fondo scala del quadrante. */
  value: number | null;
  max: number;
  level: Level;
  /** Cosa si scrive al centro (il valore, di norma). */
  display?: string;
  /** Una riga sotto il nome, per dire che cosa significa. */
  hint?: string;
}

/** Quadrante a semicerchio: l'arco si colora secondo il giudizio (verde, ambra, rosso) e brilla. */
export function Gauge({ label, value, max, level, display, hint }: Props) {
  const fraction = value === null ? 0 : value / max;
  return (
    <figure className={`gauge gauge--${level}`} title={hint}>
      <svg viewBox="0 0 200 122" role="img" aria-label={`${label}: ${display ?? value ?? '–'}`}>
        <path className="gauge-track" d={gaugeArc(1)} />
        <path className="gauge-value" d={gaugeArc(fraction)} />
        <text className="gauge-number" x="100" y="96" textAnchor="middle">
          {display ?? (value === null ? '–' : String(value))}
        </text>
      </svg>
      <figcaption>{label}</figcaption>
    </figure>
  );
}
