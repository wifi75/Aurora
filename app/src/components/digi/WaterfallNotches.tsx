import { useEffect, useState } from 'react';
import type { DigiNotch } from '../../api/client';
import { useT } from '../../i18n';
import { moveNotch, removeNotch, resizeNotch } from '../../lib/digiNotches';

interface Props {
  notches: DigiNotch[];
  minHz: number;
  maxHz: number;
  /** Larghezza della cascata in pixel. */
  width: number;
  /** Da dove partono le strisce (sotto la scala). */
  top: number;
  /** I notch cambiati (spostati, allargati, tolti): si mandano al motore. */
  onChange(list: DigiNotch[]): void;
}

/** I notch disegnati sulla cascata come strisce: si trascinano per spostarli, si allargano dai bordi, il tasto destro li toglie. */
export function WaterfallNotches({ notches, minHz, maxHz, width, top, onChange }: Props) {
  const t = useT();
  // mentre si trascina la striscia segue il mouse; al rilascio il valore va al motore e torna con l'evento
  const [draft, setDraft] = useState<DigiNotch[] | null>(null);
  useEffect(() => setDraft(null), [notches]);
  const shown = draft ?? notches;
  const xOf = (hz: number) => ((hz - minHz) / (maxHz - minHz)) * width;

  const drag = (e: React.PointerEvent<HTMLElement>, index: number, kind: 'move' | 'edge') => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    const box = (e.currentTarget.closest('.digi-wf') ?? e.currentTarget).getBoundingClientRect();
    const hzAt = (clientX: number) => minHz + ((clientX - box.left) / Math.max(1, box.width)) * (maxHz - minHz);
    const base = shown;
    const orig = base[index];
    if (!orig) return;
    const startHz = hzAt(e.clientX);
    let next = base;
    const move = (m: PointerEvent) => {
      const hz = hzAt(m.clientX);
      next = kind === 'move' ? moveNotch(base, index, orig.hz + hz - startHz) : resizeNotch(base, index, hz);
      setDraft(next);
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      if (next !== base) onChange(next);
      else setDraft(null);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  };

  return (
    <>
      {shown.map((n, i) => {
        const left = xOf(n.hz - n.width_hz / 2);
        const right = xOf(n.hz + n.width_hz / 2);
        if (right < 0 || left > width) return null;
        const w = Math.max(4, right - left);
        return (
          <span
            key={i}
            className="digi-wf-notch"
            data-testid="digi-notch"
            style={{ left: Math.min(left, xOf(n.hz) - w / 2), width: w, top }}
            title={t('digi.notch.tip', { hz: Math.round(n.hz), width: Math.round(n.width_hz) })}
            onPointerDown={(e) => drag(e, i, 'move')}
            onContextMenu={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onChange(removeNotch(shown, i));
            }}
          >
            <i className="digi-wf-notch-edge digi-wf-notch-edge--l" onPointerDown={(e) => drag(e, i, 'edge')} />
            <i className="digi-wf-notch-edge digi-wf-notch-edge--r" onPointerDown={(e) => drag(e, i, 'edge')} />
          </span>
        );
      })}
    </>
  );
}
