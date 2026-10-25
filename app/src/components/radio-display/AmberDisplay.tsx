import { formatRadioFrequency, hasFrequency, litCount, mainMeter, meterReadouts } from '../../lib/radioDisplay';
import { DisplayFrame, Label, linkState, polar, type StyleProps } from './instruments';
import { SevenSegment } from './SevenSegment';

const AMBER = '#ffb41f';
const SOFT = '#ffc241';
const DIM = '#7a5a1a';
const TICKS = 22;

/** Stile 4, LCD ambra alla Kenwood: segmenti ambra su fondo caldo e S-meter ad arco di segmenti. */
export function AmberDisplay({ model, noRadio, ariaLabel }: StyleProps) {
  const known = hasFrequency(model.dialHz);
  const link = linkState(model);
  const main = mainMeter(model);
  const lit = litCount(main.fraction, TICKS);
  const flags: Array<{ text: string; on: boolean }> = [
    { text: model.mode || '—', on: true },
    { text: 'VFO A', on: true },
    { text: 'RX', on: link.rx },
    { text: 'TX', on: link.tx },
    { text: 'DEC', on: link.busy },
    { text: 'TX EN', on: model.txEnabled },
  ];
  const line = meterReadouts(model.meters)
    .map((r) => `${r.key} ${r.text}`)
    .join('   ');
  return (
    <DisplayFrame width={680} height={150} screen="#2b1d06" ariaLabel={ariaLabel}>
      <rect x={22} y={20} width={636} height={110} rx={6} fill="#3a2808" />
      {known ? (
        flags.map((f, i) => <Label key={f.text} x={34 + i * 70} y={36} text={f.text} fill={f.on ? SOFT : DIM} size={10} weight={600} />)
      ) : (
        <Label x={34} y={36} text={noRadio} fill={SOFT} size={11} weight={600} />
      )}
      <SevenSegment text={formatRadioFrequency(model.dialHz)} x={44} y={46} w={36} h={64} t={6} gap={9} skew={0} on={AMBER} glow="#ff9a00" />
      <g aria-hidden="true">
        {Array.from({ length: TICKS }, (_, i) => {
          const a = 150 - i * 5.6;
          const [x0, y0] = polar(560, 140, 52, a);
          const [x1, y1] = polar(560, 140, 68, a);
          return <line key={i} x1={x0} y1={y0} x2={x1} y2={y1} stroke={i >= TICKS * 0.6 ? '#ff5a1a' : AMBER} strokeWidth={4} opacity={i < lit ? 1 : 0.16} />;
        })}
      </g>
      <Label x={520} y={116} text={main.kind === 'S' ? 'S 1 3 5 7 9' : 'PO'} fill={SOFT} size={10} weight={600} />
      {main.label ? <Label x={590} y={106} text={main.label} fill={SOFT} size={16} weight={600} /> : null}
      {line ? <Label x={44} y={124} text={line} fill={SOFT} size={10} /> : null}
    </DisplayFrame>
  );
}
