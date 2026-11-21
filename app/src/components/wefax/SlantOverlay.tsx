import { useEffect, useRef, useState } from 'react';
import { type WefaxSlantPoints } from '../../api/client';
import { useT } from '../../i18n';
import { pointInImage } from '../../lib/wefax';

/**
 * Lo strato sopra la cartina per la misura dell'inclinazione: due clic ai capi di una linea che dovrebbe essere verticale. Va in un
 * contenitore `position: relative` grande quanto l'immagine (`width` x `height` sono i pixel veri dell'immagine). Dopo il secondo clic
 * chiama `onMeasure` con i due punti in pixel dell'immagine; Esc annulla.
 */
export function SlantOverlay({ width, height, onMeasure, onCancel }: { width: number; height: number; onMeasure: (p: WefaxSlantPoints) => void; onCancel: () => void }) {
  const t = useT();
  const box = useRef<HTMLDivElement>(null);
  const [first, setFirst] = useState<{ x: number; y: number } | null>(null);
  const [hover, setHover] = useState<{ x: number; y: number } | null>(null);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onCancel();
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, [onCancel]);
  const at = (e: { clientX: number; clientY: number }) => {
    const el = box.current;
    return el ? pointInImage(e.clientX, e.clientY, el.getBoundingClientRect(), width, height) : null;
  };
  const pct = (p: { x: number; y: number }) => ({ left: `${(p.x / width) * 100}%`, top: `${(p.y / Math.max(1, height)) * 100}%` });
  const end = first && hover ? hover : null;
  return (
    <div
      ref={box}
      className="wefax-slant-layer"
      role="application"
      aria-label={t('wefax.slant.measureTip')}
      onMouseMove={(e) => setHover(at(e))}
      onClick={(e) => {
        const p = at(e);
        if (!p) return;
        if (!first) {
          setFirst(p);
          return;
        }
        onMeasure({ x1: first.x, y1: first.y, x2: p.x, y2: p.y });
        setFirst(null);
      }}
    >
      {first ? <i className="wefax-slant-dot" style={pct(first)} /> : null}
      {first && end ? (
        <svg className="wefax-slant-line" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
          <line x1={(first.x / width) * 100} y1={(first.y / Math.max(1, height)) * 100} x2={(end.x / width) * 100} y2={(end.y / Math.max(1, height)) * 100} vectorEffect="non-scaling-stroke" />
        </svg>
      ) : null}
      <span className="wefax-slant-hint" role="status">
        {first ? t('wefax.slant.second') : t('wefax.slant.first')}
      </span>
    </div>
  );
}
