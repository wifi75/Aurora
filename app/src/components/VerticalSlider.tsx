import { useRef, type KeyboardEvent, type PointerEvent } from 'react';

interface Props {
  /** 0-100: in alto 100, in basso 0. */
  value: number;
  label: string;
  disabled?: boolean;
  /** Disteso in orizzontale (a sinistra 0, a destra 100): per le righe compatte. */
  horizontal?: boolean;
  onChange(value: number): void;
  /** Al rilascio del mouse o del dito e alla fine di un tasto. */
  onCommit(value: number): void;
}

const clamp = (v: number) => Math.min(100, Math.max(0, Math.round(v)));

/**
 * Cursore verticale tutto nostro: il valore si ricava dalla posizione del puntatore sul binario (in alto il massimo),
 * cosi' non dipende da come un browser disegna un `<input type="range">` verticale o ruotato.
 */
export function VerticalSlider({ value, label, disabled = false, horizontal = false, onChange, onCommit }: Props) {
  const track = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);
  const last = useRef(value);

  const fromPointer = (e: PointerEvent): number => {
    const rect = track.current?.getBoundingClientRect();
    if (!rect || rect.height === 0) return value;
    if (horizontal) return rect.width === 0 ? value : clamp(((e.clientX - rect.left) / rect.width) * 100);
    return clamp((1 - (e.clientY - rect.top) / rect.height) * 100);
  };

  const move = (next: number) => {
    last.current = next;
    onChange(next);
  };

  const onKeyDown = (e: KeyboardEvent) => {
    const step = { ArrowUp: 1, ArrowRight: 1, ArrowDown: -1, ArrowLeft: -1, PageUp: 10, PageDown: -10 }[e.key];
    if (step !== undefined) {
      e.preventDefault();
      move(clamp(value + step));
    } else if (e.key === 'Home' || e.key === 'End') {
      e.preventDefault();
      move(e.key === 'End' ? 100 : 0);
    }
  };

  return (
    <div
      ref={track}
      className={`vslider${horizontal ? ' vslider--h' : ''}`}
      role="slider"
      tabIndex={disabled ? -1 : 0}
      aria-label={label}
      aria-orientation={horizontal ? 'horizontal' : 'vertical'}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={value}
      aria-disabled={disabled}
      onPointerDown={(e) => {
        if (disabled) return;
        dragging.current = true;
        e.currentTarget.setPointerCapture?.(e.pointerId);
        move(fromPointer(e));
      }}
      onPointerMove={(e) => {
        if (dragging.current && !disabled) move(fromPointer(e));
      }}
      onPointerUp={() => {
        if (!dragging.current) return;
        dragging.current = false;
        onCommit(last.current);
      }}
      onPointerCancel={() => {
        dragging.current = false;
      }}
      onKeyDown={disabled ? undefined : onKeyDown}
      onKeyUp={(e) => {
        if (!disabled && ['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'PageUp', 'PageDown', 'Home', 'End'].includes(e.key)) onCommit(last.current);
      }}
    >
      <span className="vslider-fill" style={horizontal ? { width: `${value}%` } : { height: `${value}%` }} />
      <span className="vslider-thumb" style={horizontal ? { left: `${value}%` } : { bottom: `${value}%` }} />
    </div>
  );
}
