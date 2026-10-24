import { useRef, type KeyboardEvent, type PointerEvent as ReactPointerEvent } from 'react';
import { useT } from '../i18n';
import { hzAt, leftPercent } from '../lib/markers';

interface Props {
  fMinHz: number;
  fMaxHz: number;
  /** Frequenze di ricezione e trasmissione che riporta il motore (Hz); assenti = nessun segnaposto. */
  rxHz?: number | null;
  txHz?: number | null;
  onRx(hz: number): void;
  onTx(hz: number): void;
}

const STEP = 10;
const BIG_STEP = 50;

/**
 * Le due barre del motore sopra la cascata: verde = frequenza di ricezione, rossa = trasmissione.
 * Si trascinano; un clic sulla cascata sposta la verde, Maiusc+clic la rossa.
 */
export function FrequencyMarkers({ fMinHz, fMaxHz, rxHz, txHz, onRx, onTx }: Props) {
  const t = useT();
  const box = useRef<HTMLDivElement | null>(null);

  const hzFromPointer = (clientX: number): number => {
    const r = box.current?.getBoundingClientRect();
    if (!r || r.width <= 0) return fMinHz;
    return hzAt((clientX - r.left) / r.width, fMinHz, fMaxHz);
  };

  const drag = (apply: (hz: number) => void) => (e: ReactPointerEvent<HTMLElement>) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    const handle = e.currentTarget;
    handle.setPointerCapture(e.pointerId);
    const move = (ev: PointerEvent) => apply(hzFromPointer(ev.clientX));
    const stop = () => {
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', stop);
      handle.removeEventListener('pointercancel', stop);
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', stop);
    handle.addEventListener('pointercancel', stop);
  };

  const keys = (hz: number, apply: (hz: number) => void) => (e: KeyboardEvent<HTMLElement>) => {
    const step = e.shiftKey ? BIG_STEP : STEP;
    if (e.key === 'ArrowRight') apply(Math.min(fMaxHz, hz + step));
    else if (e.key === 'ArrowLeft') apply(Math.max(fMinHz, hz - step));
    else return;
    e.preventDefault();
  };

  const marker = (kind: 'rx' | 'tx', hz: number | null | undefined, apply: (hz: number) => void, name: string) =>
    hz == null ? null : (
      <div
        key={kind}
        className={`wf-marker wf-marker--${kind}`}
        style={{ left: `${leftPercent(hz, fMinHz, fMaxHz)}%` }}
        role="slider"
        tabIndex={0}
        aria-label={name}
        title={kind === 'rx' ? t('tip.marker.rx') : t('tip.marker.tx')}
        aria-valuemin={fMinHz}
        aria-valuemax={fMaxHz}
        aria-valuenow={hz}
        onPointerDown={drag(apply)}
        onKeyDown={keys(hz, apply)}
      >
        <span className="wf-marker-tag">{`${kind === 'rx' ? 'Rx' : 'Tx'} ${hz}`}</span>
        <span className="wf-marker-line" />
      </div>
    );

  return (
    <div ref={box} className="wf-markers">
      <div
        className="wf-pad"
        onPointerDown={(e) => {
          if (e.button !== 0) return;
          const hz = hzFromPointer(e.clientX);
          if (e.shiftKey) onTx(hz);
          else onRx(hz);
        }}
      />
      {marker('rx', rxHz, onRx, t('marker.rx'))}
      {marker('tx', txHz, onTx, t('marker.tx'))}
    </div>
  );
}
