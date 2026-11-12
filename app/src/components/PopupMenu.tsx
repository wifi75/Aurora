import { useEffect, useRef, type ReactNode } from 'react';
import { createPortal } from 'react-dom';

/**
 * Un menu del tasto destro aperto in un punto della finestra (il menu dei nominativi e quello dei titoli delle colonne): sta sopra a tutto,
 * resta dentro la finestra anche aperto vicino al bordo destro o in fondo, e si chiude con Esc, con un clic fuori o quando la finestra
 * cambia (perde il fuoco o cambia misura). `at` nullo: chiuso.
 */
export function PopupMenu({
  at,
  onClose,
  className,
  width,
  height,
  label,
  children,
}: {
  at: { x: number; y: number } | null;
  onClose(): void;
  className: string;
  /** Ingombro del menu, per tenerlo dentro la finestra. */
  width: number;
  height: number;
  label?: string;
  children: ReactNode;
}) {
  const box = useRef<HTMLDivElement | null>(null);
  const open = at !== null;
  useEffect(() => {
    if (!open) return;
    const out = (e: PointerEvent) => {
      if (!box.current?.contains(e.target as Node)) onClose();
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('pointerdown', out);
    window.addEventListener('keydown', key);
    window.addEventListener('blur', onClose);
    window.addEventListener('resize', onClose);
    return () => {
      document.removeEventListener('pointerdown', out);
      window.removeEventListener('keydown', key);
      window.removeEventListener('blur', onClose);
      window.removeEventListener('resize', onClose);
    };
  }, [open, onClose]);
  if (!at) return null;
  const left = Math.max(4, Math.min(at.x, window.innerWidth - width));
  const top = Math.max(4, Math.min(at.y, window.innerHeight - height));
  return createPortal(
    <div ref={box} className={className} role="menu" aria-label={label} style={{ left, top }}>
      {children}
    </div>,
    document.body,
  );
}
