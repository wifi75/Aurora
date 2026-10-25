import { useId } from 'react';
import {
  alcFraction,
  compFraction,
  formatRadioFrequency,
  hasFrequency,
  mainMeter,
  meterReadouts,
  powerFraction,
  sFraction,
  S_TICKS,
  S9_FRACTION,
  swrFraction,
} from '../../lib/radioDisplay';
import { arcPath, Chip, DisplayFrame, Label, linkState, polar, type StyleProps } from './instruments';
import { SevenSegment } from './SevenSegment';

const PAPER = '#f2f4f8';
const BLUE = '#58c8f0';
const CX = 230;
const CY = 300;
/** Ampiezza della scala: da 90+SPAN (sinistra) a 90-SPAN (destra). */
const SPAN = 26;

interface Row {
  name: string;
  radius: number;
  value: number | null;
  /** Etichette delle tacche: posizione 0-1 e testo. */
  ticks: ReadonlyArray<{ label: string; fraction: number }>;
  /** Da questa posizione la scala cambia colore (zona rossa o dei decibel). */
  hot?: number;
}

const at = (fraction: number) => 90 + SPAN - fraction * 2 * SPAN;

/** Stile 2, strumento analogico: scale S, PO, COMP, SWR e ALC, lancetta rossa, frequenza digitale a lato. */
export function AnalogDisplay({ model, noRadio, ariaLabel }: StyleProps) {
  const clipId = `rd-analog-${useId().replace(/:/g, '')}`;
  const known = hasFrequency(model.dialHz);
  const link = linkState(model);
  const main = mainMeter(model);
  const m = model.meters;
  const rows: Row[] = [
    { name: 'S', radius: 272, value: sFraction(m), ticks: S_TICKS, hot: S9_FRACTION },
    {
      name: 'PO',
      radius: 252,
      value: powerFraction(m),
      ticks: [0, 0.25, 0.5, 0.75, 1].map((f) => ({ label: String(Math.round(f * 100)), fraction: f })),
    },
    {
      name: 'COMP',
      radius: 232,
      value: compFraction(m),
      ticks: [0, 1 / 3, 2 / 3, 1].map((f) => ({ label: String(Math.round(f * 30)), fraction: f })),
    },
    {
      name: 'SWR',
      radius: 212,
      value: swrFraction(m),
      ticks: [
        { label: '1', fraction: 0 },
        { label: '1.5', fraction: 0.25 },
        { label: '2', fraction: 0.5 },
        { label: '3', fraction: 0.75 },
        { label: '∞', fraction: 1 },
      ],
      hot: 0.5,
    },
    {
      name: 'ALC',
      radius: 192,
      value: alcFraction(m),
      ticks: [0, 0.5, 1].map((f) => ({ label: String(f), fraction: f })),
      hot: 0.8,
    },
  ];
  const readouts = meterReadouts(m).filter((r) => r.key === 'Id' || r.key === 'Vdd');
  return (
    <DisplayFrame width={680} height={150} screen="#0a0a0d" ariaLabel={ariaLabel}>
      <defs>
        <clipPath id={clipId}>
          <rect x={22} y={20} width={416} height={112} rx={6} />
        </clipPath>
      </defs>
      <rect x={20} y={18} width={420} height={116} rx={8} fill="#05060a" stroke="#7d838c" strokeWidth={2} />
      <g clipPath={`url(#${clipId})`} aria-hidden="true">
        {rows.map((row) => {
          const dim = row.value === null ? 0.35 : 1;
          const [lx, ly] = polar(CX, CY, row.radius, 90 + SPAN);
          return (
            <g key={row.name} opacity={dim}>
              <path d={arcPath(CX, CY, row.radius, 90 + SPAN, 90 - SPAN)} fill="none" stroke={PAPER} strokeWidth={1.4} />
              {row.hot !== undefined ? <path d={arcPath(CX, CY, row.radius, at(row.hot), 90 - SPAN)} fill="none" stroke={row.name === 'S' ? BLUE : '#ff6a4a'} strokeWidth={4} /> : null}
              {row.ticks.map((tick) => {
                const [x0, y0] = polar(CX, CY, row.radius, at(tick.fraction));
                const [x1, y1] = polar(CX, CY, row.radius + 5, at(tick.fraction));
                const [tx, ty] = polar(CX, CY, row.radius - 8, at(tick.fraction));
                return (
                  <g key={tick.label}>
                    <line x1={x0} y1={y0} x2={x1} y2={y1} stroke={PAPER} strokeWidth={1.2} />
                    <Label x={tx} y={ty + 3} text={tick.label} fill={row.hot !== undefined && tick.fraction >= row.hot && row.name === 'S' ? BLUE : PAPER} size={8} weight={600} anchor="middle" />
                  </g>
                );
              })}
              <Label x={lx - 6} y={ly + 3} text={row.name} fill={PAPER} size={9} weight={600} anchor="end" />
              {row.value !== null ? <circle cx={polar(CX, CY, row.radius, at(row.value))[0]} cy={polar(CX, CY, row.radius, at(row.value))[1]} r={2.8} fill={BLUE} /> : null}
            </g>
          );
        })}
        {main.fraction !== null ? (
          <line
            x1={CX}
            y1={CY}
            x2={polar(CX, CY, 280, at(main.fraction))[0]}
            y2={polar(CX, CY, 280, at(main.fraction))[1]}
            stroke="#e8231c"
            strokeWidth={2.2}
          />
        ) : null}
      </g>
      <SevenSegment text={formatRadioFrequency(model.dialHz)} x={452} y={30} w={21} h={38} t={3.6} gap={6} skew={0.08} on="#7be3ff" glow="#3fa0c8" />
      {known ? <Label x={456} y={90} text={model.mode || '—'} fill="#7be3ff" size={13} weight={600} /> : <Label x={456} y={90} text={noRadio} fill="#7be3ff" size={11} />}
      {readouts.length > 0 ? <Label x={520} y={90} text={readouts.map((r) => `${r.key} ${r.text}`).join('  ')} fill="#7be3ff" size={10} /> : null}
      <Chip x={456} y={104} w={34} h={18} text="RX" bg="#1f7a2a" fg="#d4ffd9" fontSize={11} rx={4} dim={!link.rx} />
      <Chip x={496} y={104} w={34} h={18} text="TX" bg="#c0301f" fg="#ffe3de" fontSize={11} rx={4} dim={!link.tx} />
      <Chip x={536} y={104} w={44} h={18} text="DEC" bg="#2a2f36" fg="#a7b0bb" fontSize={10} rx={4} dim={!link.busy} />
      {main.label ? <Label x={652} y={122} text={main.kind === "S" ? main.label : `PO ${main.label}`} fill="#7be3ff" size={12} weight={600} anchor="end" /> : null}
    </DisplayFrame>
  );
}
