import { useState } from 'react';

// altezze regolabili a mano (cascata della Fox e del Contest): si ricordano nel browser
const readHeight = (key: string, fallback: number): number => {
  try {
    const v = Number(window.localStorage.getItem(key));
    return v >= 120 && v <= 1600 ? v : fallback;
  } catch {
    return fallback;
  }
};
const saveHeight = (key: string, h: number) => {
  try {
    if (h >= 120) window.localStorage.setItem(key, String(h));
  } catch {
    // senza archivio del browser l'altezza non si ricorda
  }
};

/**
 * Un'altezza che si cambia trascinando una maniglia col mouse (o col dito) e che si ricorda. `onPointerDown` la fa crescere
 * trascinando in giu'; `onPointerDownInverse` la fa crescere trascinando in su (la maniglia sopra la cascata: piu' giu' si porta,
 * piu' spazio va ai riquadri e meno alla cascata).
 */
export function useDragHeight(key: string, fallback: number, max = 1600) {
  const [height, setHeight] = useState(() => Math.min(max, readHeight(key, fallback)));
  const drag = (sign: 1 | -1) => (e: React.PointerEvent<HTMLDivElement>) => {
    const startY = e.clientY;
    const startH = height;
    const el = e.currentTarget;
    el.setPointerCapture?.(e.pointerId);
    const at = (y: number) => Math.min(max, Math.max(120, startH + sign * (y - startY)));
    const move = (ev: PointerEvent) => setHeight(at(ev.clientY));
    const up = (ev: PointerEvent) => {
      el.removeEventListener('pointermove', move);
      el.removeEventListener('pointerup', up);
      saveHeight(key, at(ev.clientY));
    };
    el.addEventListener('pointermove', move);
    el.addEventListener('pointerup', up);
  };
  return { height, onPointerDown: drag(1), onPointerDownInverse: drag(-1) };
}

/** La maniglia orizzontale per trascinare un'altezza; `className` sceglie lo stile della pagina. */
export function DragHandle({ label, className = 'foxc-wf-handle', onPointerDown }: { label: string; className?: string; onPointerDown(e: React.PointerEvent<HTMLDivElement>): void }) {
  return (
    <div className={className} role="separator" aria-orientation="horizontal" aria-label={label} title={label} onPointerDown={onPointerDown}>
      <span />
    </div>
  );
}
