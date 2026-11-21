import { formatRadioFrequency, hasFrequency, litCount, mainMeter, type RadioDisplayStyle } from '../../lib/radioDisplay';
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
  ft2000: { screen: '#020405', digits: '#5ce8ff', glow: '#1aa8d0', ink: '#ffb428', rx: '#47d13a', rxInk: '#052200', tx: '#ff3b2f', txInk: '#2a0500', meter: 'bar', lo: '#5ce8ff', hi: '#ff3b2f', off: '#0e2630', skew: 0.08 },
  ic7610: { screen: '#000000', digits: '#f4f6fa', ink: '#7fd4ff', rx: '#1aa0e8', rxInk: '#02060c', tx: '#ff3a2a', txInk: '#ffffff', meter: 'ruler', lo: '#f4f6fa', hi: '#ff3a2a', off: '#3a4656', skew: 0 },
  icblueq: { screen: '#061a45', digits: '#ffffff', glow: '#6f92d8', ink: '#cfe0ff', rx: '#2f6fd8', rxInk: '#ffffff', tx: '#c62828', txInk: '#ffffff', meter: 'bar', lo: '#ffffff', hi: '#ff5252', off: '#03133a', skew: 0 },
  icblackq: { screen: '#000000', digits: '#f4f6fa', ink: '#7fd4ff', rx: '#1aa0e8', rxInk: '#02060c', tx: '#ff3a2a', txInk: '#ffffff', meter: 'ruler', lo: '#f4f6fa', hi: '#ff3a2a', off: '#3a4656', skew: 0 },
  yaesuq: { screen: '#101010', digits: '#fac775', glow: '#ef9f27', ink: '#ef9f27', rx: '#fac775', rxInk: '#241604', tx: '#e24b4a', txInk: '#ffffff', meter: 'ramp', lo: '#fac775', hi: '#f09595', off: '#1f1a12', skew: 0 },
  led: { screen: '#0b0505', digits: '#ff3a22', glow: '#ff2a10', ink: '#ff7a5a', rx: '#3fd13a', rxInk: '#032200', tx: '#ff3a22', txInk: '#2a0500', meter: 'led', lo: '#3fd13a', hi: '#ff3a22', off: '#3a1410', skew: 0.1 },
};

/** Colonna di destra: RX/TX, misuratore a 10 tacche uguali (largo quanto la colonna) e, in trasmissione, la barra SWR sotto quella della potenza. */
const MX = 168;
const BAR_STEP = 8.2;
const BAR_SEG = 6.4;
const BAR_COUNT = 10;
const TEXT_X = 252;

/** Il ROS da 1 a 3 sulla barra: 1,0 vuota, 3,0 piena. */
function swrFraction(swr: number): number {
  return Math.min(1, Math.max(0, (swr - 1) / 2));
}

/** Display piccolo (testata e telefono): frequenza con il VFO B sotto, RX/TX piccoli, misuratore uniforme; in trasmissione potenza e ROS in due barre. */
export function MiniDisplay({ model, ariaLabel, style, vfoB = null }: StyleProps & { style: RadioDisplayStyle; vfoB?: string | null }) {
  const p = PALETTES[style];
  const known = hasFrequency(model.dialHz);
  const link = linkState(model);
  const main = mainMeter(model);
  const sending = known && link.tx;
  const swr = typeof model.meters?.swr === 'number' ? model.meters.swr : null;
  const swrColor = swr !== null && swr >= 3 ? '#ff4a3a' : swr !== null && swr >= 2 ? '#ffb020' : '#5be08a';
  const off = p.off;
  return (
    <DisplayFrame width={306} height={52} radius={8} inset={3} stretch screen={p.screen} ariaLabel={ariaLabel}>
      {/* con il VFO B noto la frequenza principale e' piu' bassa e sotto resta una riga piccola con il VFO B */}
      {vfoB ? (
        <>
          <SevenSegment text={formatRadioFrequency(model.dialHz)} x={11} y={8} w={11} h={21} t={2.3} gap={3} skew={p.skew} on={p.digits} glow={p.glow} offOpacity={0.14} />
          <Label x={11} y={44} text={`VFOB ${vfoB}`} fill={p.ink} size={9} weight={600} />
        </>
      ) : (
        <SevenSegment text={formatRadioFrequency(model.dialHz)} x={11} y={13} w={13} h={26} t={2.8} gap={3.5} skew={p.skew} on={p.digits} glow={p.glow} offOpacity={0.14} />
      )}
      {/* TUNE e' un pulsante vero sopra il disegno (vedi RadioDisplayHeader), tra la frequenza e RX, sulla riga della barra in basso */}
      <Chip x={MX} y={5.5} w={24} h={11} text="RX" bg={p.rx} fg={p.rxInk} fontSize={9} rx={3} dim={!link.rx} />
      <Chip x={MX + 28} y={5.5} w={24} h={11} text="TX" bg={p.tx} fg={p.txInk} fontSize={9} rx={3} dim={!link.tx} />
      {sending ? (
        <>
          {/* in trasmissione il misuratore diventa quello della potenza e sotto c'e' la barra del ROS, ognuna con il suo testo alla stessa altezza */}
          <SegBar x={MX} y={22} count={BAR_COUNT} segW={BAR_SEG} segH={6} step={BAR_STEP} lit={litCount(main.fraction, BAR_COUNT)} color={() => '#ff3b2f'} off={off} offOpacity={0.18} />
          <Label x={TEXT_X} y={28.5} text={`PWR ${typeof model.meters?.power_w === 'number' ? `${Math.round(model.meters.power_w)} W` : '–'}`} fill="#ff3b2f" size={10} weight={700} />
          <SegBar x={MX} y={36} count={BAR_COUNT} segW={BAR_SEG} segH={6} step={BAR_STEP} lit={swr === null ? 0 : litCount(swrFraction(swr), BAR_COUNT)} color={() => swrColor} off={off} offOpacity={0.18} />
          <Label x={TEXT_X} y={42.5} text={`SWR ${swr === null ? '–' : swr.toFixed(1)}`} fill={swrColor} size={10} weight={700} />
        </>
      ) : (
        <>
          <SegBar x={MX} y={22} count={BAR_COUNT} segW={BAR_SEG} segH={6} step={BAR_STEP} lit={litCount(main.fraction, BAR_COUNT)} color={() => p.digits} off={off} offOpacity={0.18} />
          {main.label && main.kind === 'S' ? <Label x={TEXT_X} y={28.5} text={main.label} fill={p.ink} size={10} weight={600} /> : null}
        </>
      )}
    </DisplayFrame>
  );
}
