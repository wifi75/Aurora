import { useEffect, useRef, type MouseEvent as ReactMouseEvent } from 'react';
import { createPortal } from 'react-dom';
import { useT } from '../i18n';
import { qrzUrl } from '../lib/callsignPage';
import '../styles/call-menu.css';

/** Il menu aperto: su quale nominativo, dove, e la voce «Metti in coda» se in quel punto si puo'. */
export interface CallMenuState {
  call: string;
  x: number;
  y: number;
  onQueue?: () => void;
  /** La stazione e' gia' in coda: la voce la toglie. */
  queued?: boolean;
}

/** Dove aprire il menu: il punto del clic, o sotto l'elemento se si e' usato il tasto Menu della tastiera (punto 0,0). */
export function callMenuAt(e: ReactMouseEvent<HTMLElement>, call: string, onQueue?: () => void, queued = false): CallMenuState {
  let { clientX: x, clientY: y } = e;
  if (x === 0 && y === 0) {
    const r = (e.target as HTMLElement).getBoundingClientRect();
    x = r.left;
    y = r.bottom;
  }
  return { call, x, y, onQueue, queued };
}

/**
 * Il menu del tasto destro su un nominativo (Attivita' di banda, finestra Rx, DX Cluster): «Apri su QRZ.com» apre la scheda nel browser
 * e, dove la riga si puo' mettere in coda, «Metti in coda». Si chiude con Esc, con un clic fuori o quando la finestra cambia.
 */
export function CallMenu({ menu, onClose }: { menu: CallMenuState | null; onClose(): void }) {
  const t = useT();
  const box = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    if (!menu) return;
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
  }, [menu, onClose]);
  if (!menu) return null;
  const href = qrzUrl(menu.call);
  if (!href) return null;
  // vicino al bordo destro o in basso il menu si apre verso l'interno, per restare tutto visibile
  const left = Math.max(4, Math.min(menu.x, window.innerWidth - 200));
  const top = Math.max(4, Math.min(menu.y, window.innerHeight - 90));
  return createPortal(
    <div ref={box} className="call-menu" role="menu" aria-label={menu.call} style={{ left, top }}>
      <a role="menuitem" href={href} target="_blank" rel="noreferrer noopener" title={t('callmenu.qrz.tip', { call: menu.call })} autoFocus onClick={onClose}>
        {t('callmenu.qrz')}
      </a>
      {menu.onQueue ? (
        <button
          type="button"
          role="menuitem"
          onClick={() => {
            menu.onQueue?.();
            onClose();
          }}
        >
          {t(menu.queued ? 'callmenu.unqueue' : 'callmenu.queue')}
        </button>
      ) : null}
    </div>,
    document.body,
  );
}
