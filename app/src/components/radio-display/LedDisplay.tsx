import { formatRadioFrequency, hasFrequency, litCount, meterReadouts, powerFraction, sFraction, sLabel } from '../../lib/radioDisplay';
import { Chip, DisplayFrame, Label, linkState, SegBar, type StyleProps } from './instruments';
import { SevenSegment } from './SevenSegment';

const RED = '#ff3a22';
const SEGMENTS = 32;

/** Colore di una tacca: verde fino a 18, giallo fino a 27, poi rosso. */
const zone = (i: number) => (i < 18 ? '#3fd13a' : i < 27 ? '#ffd23a' : RED);

/** Stile 5, LED rosso classico: cifre rosse e barre a segmenti verde, giallo e rosso, leggibili da lontano. */
export function LedDisplay({ model, noRadio, ariaLabel }: StyleProps) {
  const known = hasFrequency(model.dialHz);
  const link = linkState(model);
  const m = model.meters;
  const power = meterReadouts(m)
    .filter((r) => r.key === 'PO' || r.key === 'SWR')
    .map((r) => `${r.key} ${r.text}`)
    .join('  ');
  return (
    <DisplayFrame width={680} height={150} screen="#0b0505" ariaLabel={ariaLabel}>
      <SevenSegment text={formatRadioFrequency(model.dialHz)} x={40} y={24} w={40} h={70} t={6.2} gap={10} skew={0.1} on={RED} glow="#ff2a10" />
      {known ? <Label x={430} y={50} text={model.mode || '—'} fill="#ff7a5a" size={16} weight={600} /> : <Label x={430} y={50} text={noRadio} fill="#ff7a5a" size={11} weight={600} />}
      <Chip x={500} y={30} w={40} h={20} text="RX" bg="#3fd13a" fg="#032200" fontSize={11} rx={4} dim={!link.rx} />
      <Chip x={550} y={30} w={40} h={20} text="TX" bg="#ff3a22" fg="#2a0500" fontSize={11} rx={4} dim={!link.tx} />
      <Label x={30} y={113} text="S" fill="#ffb09a" size={10} weight={600} />
      <SegBar x={62} y={104} count={SEGMENTS} segW={12} segH={12} step={16} lit={litCount(sFraction(m), SEGMENTS)} color={zone} off="#3a1410" rx={2} />
      <Label x={580} y={114} text={sLabel(m) ?? ''} fill="#ffd23a" size={12} weight={600} />
      <Label x={30} y={131} text="PWR" fill="#ffb09a" size={10} weight={600} />
      <SegBar x={62} y={122} count={SEGMENTS} segW={12} segH={12} step={16} lit={litCount(powerFraction(m), SEGMENTS)} color={zone} off="#3a1410" rx={2} />
      <Label x={578} y={132} text={power} fill="#ffd23a" size={9} />
    </DisplayFrame>
  );
}
