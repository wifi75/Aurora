import { formatRadioFrequency, hasFrequency, litCount, meterReadouts, powerFraction, sFraction, sLabel } from '../../lib/radioDisplay';
import { DisplayFrame, Label, linkState, SegBar, type StyleProps } from './instruments';
import { SevenSegment } from './SevenSegment';

const INK = '#9fd2ff';
const CYAN = '#44e8ff';
const BAR_SEGMENTS = 30;

/** Stile 3, schermo a colori alla Icom: cifre turchesi, riga di stato in alto, barre orizzontali S e potenza. */
export function ColorDisplay({ model, noRadio, ariaLabel }: StyleProps) {
  const known = hasFrequency(model.dialHz);
  const link = linkState(model);
  const m = model.meters;
  const readouts = meterReadouts(m);
  const byKey = (key: string) => readouts.find((r) => r.key === key)?.text;
  const comp = byKey('COMP');
  const status: Array<{ text: string; fill: string }> = [
    { text: 'VFO A', fill: INK },
    { text: model.mode || '—', fill: '#ffd34a' },
    { text: 'RX', fill: link.rx ? '#5dff8f' : '#4a6a8a' },
    { text: 'TX', fill: link.tx ? '#ff6a5a' : '#4a6a8a' },
    { text: 'DEC', fill: link.busy ? INK : '#4a6a8a' },
    ...(model.txEnabled ? [{ text: 'TX EN', fill: '#ffd34a' }] : []),
  ];
  const power = readouts.filter((r) => r.key === 'PO' || r.key === 'SWR' || r.key === 'ALC').map((r) => `${r.key} ${r.text}`).join('  ');
  return (
    <DisplayFrame width={680} height={150} screen="#07182b" ariaLabel={ariaLabel}>
      <rect x={20} y={18} width={640} height={20} rx={3} fill="#0f2b4a" />
      {known ? (
        status.map((item, i) => <Label key={item.text} x={30 + i * 68} y={32} text={item.text} fill={item.fill} size={11} weight={600} />)
      ) : (
        <Label x={30} y={32} text={noRadio} fill="#ffd34a" size={11} weight={600} />
      )}
      <SevenSegment text={formatRadioFrequency(model.dialHz, 'hz')} x={40} y={46} w={34} h={60} t={5.4} gap={9} skew={0.12} on={CYAN} glow="#1fb4d6" />
      <Label x={470} y={62} text="Id" fill={INK} size={11} />
      <Label x={470} y={82} text={byKey('Id') ?? '—'} fill={byKey('Id') ? CYAN : '#4a6a8a'} size={14} weight={600} />
      <Label x={570} y={62} text="Vdd" fill={INK} size={11} />
      <Label x={570} y={82} text={byKey('Vdd') ?? '—'} fill={byKey('Vdd') ? CYAN : '#4a6a8a'} size={14} weight={600} />
      <Label x={30} y={124} text="S" fill={INK} size={10} />
      <SegBar x={70} y={114} count={BAR_SEGMENTS} segW={11} segH={9} step={14} lit={litCount(sFraction(m), BAR_SEGMENTS)} color={(i) => (i >= BAR_SEGMENTS * 0.6 ? '#ff5a3a' : CYAN)} off="#123257" />
      <Label x={500} y={122} text={[sLabel(m), comp ? `COMP ${comp}` : null].filter(Boolean).join('  ')} fill="#ffd34a" size={11} weight={600} />
      <Label x={30} y={136} text="PO" fill={INK} size={10} />
      <SegBar x={70} y={126} count={BAR_SEGMENTS} segW={11} segH={9} step={14} lit={litCount(powerFraction(m), BAR_SEGMENTS)} color={(i) => (i >= BAR_SEGMENTS * 0.8 ? '#ff5a3a' : '#8dff6a')} off="#123257" />
      <Label x={500} y={134} text={power} fill="#8dff6a" size={10} />
    </DisplayFrame>
  );
}
