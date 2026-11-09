import { useEffect, useState } from 'react';
import { browserStorage } from './theme';

/** Dove compare l'avviso in evidenza del DXCC nuovo: in alto a destra di fabbrica. */
export const NEWDX_POSITIONS = ['top-right', 'top-left', 'top-center', 'bottom-right', 'bottom-left', 'bottom-center'] as const;
export type NewDxPos = (typeof NEWDX_POSITIONS)[number];

const KEY = 'aurora-newdx-pos';
const EVENT = 'aurora-newdx-pos';

function read(): NewDxPos {
  try {
    const v = browserStorage()?.getItem(KEY);
    return (NEWDX_POSITIONS as readonly string[]).includes(v ?? '') ? (v as NewDxPos) : 'top-right';
  } catch {
    return 'top-right';
  }
}

export function setNewDxPos(pos: NewDxPos): void {
  try {
    browserStorage()?.setItem(KEY, pos);
  } catch {
    /* senza memoria del browser vale solo per questa volta */
  }
  window.dispatchEvent(new Event(EVENT));
}

/** La posizione scelta; si aggiorna anche quando la si cambia da un'altra finestra (Impostazioni staccate). */
export function useNewDxPos(): NewDxPos {
  const [pos, setPos] = useState<NewDxPos>(read);
  useEffect(() => {
    const update = () => setPos(read());
    window.addEventListener(EVENT, update);
    window.addEventListener('storage', update);
    return () => {
      window.removeEventListener(EVENT, update);
      window.removeEventListener('storage', update);
    };
  }, []);
  return pos;
}
