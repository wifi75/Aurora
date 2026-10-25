import { formatRadioFrequency, hasFrequency, litCount, mainMeter, S_TICKS } from '../../lib/radioDisplay';
import { Chip, DisplayFrame, Label, linkState, type StyleProps } from './instruments';
import { SevenSegment } from './SevenSegment';

const WHITE = '#eef6ff';
const BLUE = '#9bc4ff';
const GREEN = '#7ee05a';
const DOTS = 20;

/** Stile 1, fluorescente alla Yaesu: cifre bianche, spie arancio, BUSY, S-meter a tacche, led RX verde e TX rosso. */
export function FluorescentDisplay({ model, noRadio, ariaLabel }: StyleProps) {
  const known = hasFrequency(model.dialHz);
  const link = linkState(model);
  const main = mainMeter(model);
  const lit = litCount(main.fraction, DOTS);
  return (
    <DisplayFrame width={680} height={150} screen="#06080b" ariaLabel={ariaLabel}>
      {/* le spie MONI, BK-IN e TUNER non arrivano dal CAT: restano spente */}
      <Chip x={150} y={18} w={38} text="MONI" bg="#ff5a1f" fg="#1a0600" dim />
      <Chip x={196} y={18} w={38} text="BK-IN" bg="#ff5a1f" fg="#1a0600" dim />
      <Chip x={242} y={18} w={48} text="TUNER" bg="#ff5a1f" fg="#1a0600" dim />
      <SevenSegment text={formatRadioFrequency(model.dialHz)} x={96} y={40} w={34} h={66} t={6} gap={9} skew={0.08} on={WHITE} glow={BLUE} />
      <Label x={30} y={34} text="VFO-A" fill={BLUE} />
      <Chip x={26} y={112} w={70} h={20} text="BUSY" bg="#cfe85a" fg="#1a2300" fontSize={12} rx={3} dim={!link.busy} />
      <rect x={540} y={24} width={112} height={44} rx={4} fill="none" stroke="#cfd7e0" strokeWidth={1.2} />
      {known ? (
        <>
          <Label x={552} y={44} text={model.mode || '—'} fill={WHITE} size={16} weight={600} />
          {model.txEnabled ? <Label x={552} y={60} text="TX EN" fill="#cfe85a" size={11} /> : null}
        </>
      ) : (
        <Label x={552} y={50} text={noRadio} fill="#cfe85a" size={11} />
      )}
      <Label x={430} y={100} text={main.kind === 'S' ? 'S' : 'PO'} fill={WHITE} size={13} weight={600} />
      <g aria-hidden="true">
        {Array.from({ length: DOTS }, (_, i) => {
          const on = i < lit;
          const tall = i <= 11;
          return <rect key={i} x={448 + i * 10} y={tall ? 90 : 96} width={6} height={tall ? 18 : 12} fill={on ? (i > 11 ? GREEN : WHITE) : WHITE} opacity={on ? 1 : 0.13} />;
        })}
      </g>
      {main.kind === 'S'
        ? S_TICKS.filter((tick) => tick.label !== '+60').map((tick) => (
            <Label key={tick.label} x={448 + Math.round(tick.fraction * DOTS) * 10} y={128} text={tick.label} fill={BLUE} size={9} />
          ))
        : null}
      {main.label ? <Label x={448} y={84} text={main.label} fill={WHITE} size={12} weight={600} /> : null}
      <Chip x={580} y={70} w={36} h={16} text="RX" bg="#47d13a" fg="#052200" fontSize={10} rx={4} dim={!link.rx} />
      <Chip x={620} y={70} w={36} h={16} text="TX" bg="#e8402a" fg="#2a0500" fontSize={10} rx={4} dim={!link.tx} />
    </DisplayFrame>
  );
}
