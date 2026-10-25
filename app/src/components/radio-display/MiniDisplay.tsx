import { formatRadioFrequency, hasFrequency, litCount, mainMeter, S9_FRACTION, type RadioDisplayStyle } from '../../lib/radioDisplay';
import { Chip, DisplayFrame, Label, linkState, SegBar, type StyleProps } from './instruments';
import { SevenSegment } from './SevenSegment';

/** Versione compatta (testata e telefono): frequenza, modo, RX/TX e un S-meter, nei colori dello stile scelto. */

type MeterKind = 'dots' | 'ruler' | 'bar' | 'ramp' | 'led';

interface Palette {
  screen: string;
  digits: string;
  glow?: string;
  ink: string;
  rx: string;
  rxInk: string;
  tx: string;
  txInk: string;
  meter: MeterKind;
  /** Colore delle tacche accese sotto S9 e sopra. */
  lo: string;
  hi: string;
  off: string;
  skew: number;
}

const PALETTES: Record<RadioDisplayStyle, Palette> = {
  fluorescent: { screen: '#06080b', digits: '#eef6ff', glow: '#9bc4ff', ink: '#9bc4ff', rx: '#47d13a', rxInk: '#052200', tx: '#e8402a', txInk: '#2a0500', meter: 'dots', lo: '#eef6ff', hi: '#7ee05a', off: '#eef6ff', skew: 0.08 },
  analog: { screen: '#0a0a0d', digits: '#7be3ff', glow: '#3fa0c8', ink: '#7be3ff', rx: '#1f7a2a', rxInk: '#d4ffd9', tx: '#c0301f', txInk: '#ffe3de', meter: 'ruler', lo: '#f2f4f8', hi: '#58c8f0', off: '#f2f4f8', skew: 0.08 },
  color: { screen: '#07182b', digits: '#44e8ff', glow: '#1fb4d6', ink: '#9fd2ff', rx: '#2fbf63', rxInk: '#02210e', tx: '#ff5a3a', txInk: '#2a0800', meter: 'bar', lo: '#44e8ff', hi: '#ff5a3a', off: '#123257', skew: 0.12 },
  amber: { screen: '#3a2808', digits: '#ffb41f', glow: '#ff9a00', ink: '#ffc241', rx: '#ffb41f', rxInk: '#2b1d06', tx: '#ff5a1a', txInk: '#2b0d00', meter: 'ramp', lo: '#ffb41f', hi: '#ff5a1a', off: '#ffb41f', skew: 0 },
  led: { screen: '#0b0505', digits: '#ff3a22', glow: '#ff2a10', ink: '#ff7a5a', rx: '#3fd13a', rxInk: '#032200', tx: '#ff3a22', txInk: '#2a0500', meter: 'led', lo: '#3fd13a', hi: '#ff3a22', off: '#3a1410', skew: 0.1 },
};

const MX = 160;
const MW = 150;

function MiniMeter({ kind, p, fraction }: { kind: MeterKind; p: Palette; fraction: number | null }) {
  switch (kind) {
    case 'dots': {
      const n = 19;
      const lit = litCount(fraction, n);
      return (
        <g aria-hidden="true">
          {Array.from({ length: n }, (_, i) => (
            <rect key={i} x={MX + i * 8} y={i <= n * S9_FRACTION ? 28 : 31} width={5} height={i <= n * S9_FRACTION ? 14 : 11} fill={i < lit && i > n * S9_FRACTION ? p.hi : p.lo} opacity={i < lit ? 1 : 0.13} />
          ))}
        </g>
      );
    }
    case 'ruler': {
      // scala lineare con la lancetta rossa: tacche ogni S, la lancetta e' una riga inclinata
      const x = fraction === null ? null : MX + fraction * MW;
      return (
        <g aria-hidden="true">
          <rect x={MX} y={41} width={MW} height={1.4} fill={p.lo} opacity={fraction === null ? 0.3 : 0.9} />
          <rect x={MX + MW * S9_FRACTION} y={39.5} width={MW * (1 - S9_FRACTION)} height={3} fill={p.hi} opacity={fraction === null ? 0.3 : 1} />
          {Array.from({ length: 16 }, (_, i) => (
            <rect key={i} x={MX + (i / 15) * (MW - 1)} y={i % 2 ? 36 : 33} width={1} height={i % 2 ? 6 : 9} fill={i / 15 > S9_FRACTION ? p.hi : p.lo} opacity={fraction === null ? 0.3 : 0.9} />
          ))}
          {x !== null ? <line x1={x - 6} y1={44} x2={x + 3} y2={27} stroke="#e8231c" strokeWidth={2} strokeLinecap="round" /> : null}
        </g>
      );
    }
    case 'bar': {
      const n = 28;
      const lit = litCount(fraction, n);
      return <SegBar x={MX} y={30} count={n} segW={4} segH={11} step={MW / n} lit={lit} color={(i) => (i >= n * S9_FRACTION ? p.hi : p.lo)} off={p.off} />;
    }
    case 'ramp': {
      const n = 18;
      const lit = litCount(fraction, n);
      return (
        <g aria-hidden="true">
          {Array.from({ length: n }, (_, i) => {
            const h = 4 + (i / (n - 1)) * 13;
            return <rect key={i} x={MX + i * (MW / n)} y={43 - h} width={5} height={h} fill={i >= n * S9_FRACTION && i < lit ? p.hi : p.lo} opacity={i < lit ? 1 : 0.16} />;
          })}
        </g>
      );
    }
    default: {
      const n = 15;
      const lit = litCount(fraction, n);
      return <SegBar x={MX} y={30} count={n} segW={7} segH={11} step={MW / n} lit={lit} color={(i) => (i < n * 0.55 ? p.lo : i < n * 0.85 ? '#ffd23a' : p.hi)} off={p.off} offOpacity={1} rx={1} />;
    }
  }
}

/** Display piccolo: circa 6:1, adatto alla testata (altezza di una riga di pulsanti) e al telefono. */
export function MiniDisplay({ model, noRadio, ariaLabel, style }: StyleProps & { style: RadioDisplayStyle }) {
  const p = PALETTES[style];
  const known = hasFrequency(model.dialHz);
  const link = linkState(model);
  const main = mainMeter(model);
  return (
    <DisplayFrame width={320} height={52} radius={8} inset={3} screen={p.screen} ariaLabel={ariaLabel}>
      <SevenSegment text={formatRadioFrequency(model.dialHz)} x={11} y={13} w={13} h={26} t={2.8} gap={3.5} skew={p.skew} on={p.digits} glow={p.glow} offOpacity={0.14} />
      <Chip x={MX} y={8} w={28} h={14} text="RX" bg={p.rx} fg={p.rxInk} fontSize={10} rx={3} dim={!link.rx} />
      <Chip x={MX + 32} y={8} w={28} h={14} text="TX" bg={p.tx} fg={p.txInk} fontSize={10} rx={3} dim={!link.tx} />
      {known ? (
        <Label x={MX + 70} y={19} text={model.mode || '—'} fill={p.ink} size={12} weight={600} />
      ) : (
        <Label x={MX + 66} y={19} text={noRadio} fill={p.ink} size={10} weight={600} />
      )}
      {main.label ? <Label x={312} y={20} text={main.kind === 'S' ? main.label : `PO ${main.label}`} fill={p.ink} size={12} weight={600} anchor="end" /> : null}
      <MiniMeter kind={p.meter} p={p} fraction={main.fraction} />
    </DisplayFrame>
  );
}
