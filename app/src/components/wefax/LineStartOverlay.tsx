import { useEffect, useRef, useState } from 'react';
import { useT } from '../../i18n';
import { pointInImage } from '../../lib/wefax';

/**
 * Lo strato sopra la cartina per «Imposta inizio riga»: un clic sulla colonna che deve diventare il bordo sinistro (per esempio la cornice della
 * cartina arrivata spostata di lato). Un filo verticale segue il mouse; Esc annulla. Va in un contenitore `position: relative` grande quanto
 * l'immagine (`width` x `height` sono i pixel veri dell'immagine).
 */
export function LineStartOverlay({ width, height, onPick, onCancel }: { width: number; height: number; onPick: (x: number) => void; onCancel: () => void }) {
  const t = useT();
  const box = useRef<HTMLDivElement>(null);
  const [hover, setHover] = useState<number | null>(null);
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
  return (
    <div
      ref={box}
      className="wefax-slant-layer"
      role="application"
      aria-label={t('wefax.start.tip')}
      onMouseMove={(e) => setHover(at(e)?.x ?? null)}
      onClick={(e) => {
        const p = at(e);
        if (p) onPick(Math.round(p.x));
      }}
    >
      {hover !== null ? <i className="wefax-start-guide" style={{ left: `${(hover / width) * 100}%` }} /> : null}
      <span className="wefax-slant-hint" role="status">
        {t('wefax.start.pick')}
      </span>
    </div>
  );
}
