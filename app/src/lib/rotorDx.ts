import { useEffect, useState } from 'react';
import { browserStorage } from './theme';

/**
 * Il DX verso cui puntare il rotore, scritto dalla pagina che lo conosce (la Stazione per FT8, FT4, JT65, Q65, ...: il DX del QSO in corso;
 * PSK, RTTY e CW: il nominativo e il locatore del QSO scritti nella pagina) e letto dal rotore staccato in una finestra a parte, che da solo
 * non sa quale pagina e' aperta nella finestra principale. Passa dall'archivio del browser, condiviso fra le finestre di Aurora.
 */
export interface RotorDx {
  call: string;
  grid: string;
}

const KEY = 'aurora.rotor.dx';
const EVENT = 'aurora-rotor-dx';

export function readRotorDx(): RotorDx | null {
  try {
    const v = JSON.parse(browserStorage()?.getItem(KEY) ?? 'null') as Partial<RotorDx> | null;
    return v && typeof v.call === 'string' && typeof v.grid === 'string' ? { call: v.call, grid: v.grid } : null;
  } catch {
    return null;
  }
}

/** Scrive il DX attuale (solo se e' cambiato, per non svegliare le altre finestre a ogni giro). */
export function publishRotorDx(call: string, grid: string): void {
  const next: RotorDx = { call: call.trim().toUpperCase(), grid: grid.trim() };
  const now = readRotorDx();
  if (now && now.call === next.call && now.grid === next.grid) return;
  try {
    browserStorage()?.setItem(KEY, JSON.stringify(next));
  } catch {
    // senza archivio il rotore staccato usa il DX della Stazione
  }
  try {
    window.dispatchEvent(new Event(EVENT));
  } catch {
    // fuori dal browser (prove) non c'e' nessuno da avvisare
  }
}

/** La pagina che conosce il DX lo tiene aggiornato per il rotore (anche staccato in un'altra finestra). */
export function usePublishRotorDx(call: string, grid: string): void {
  useEffect(() => {
    publishRotorDx(call, grid);
  }, [call, grid]);
}

/** Il DX scritto dall'ultima pagina aperta, aggiornato anche quando lo cambia un'altra finestra. */
export function useRotorDx(): RotorDx | null {
  const [dx, setDx] = useState(readRotorDx);
  useEffect(() => {
    const update = (e: Event) => {
      if (e instanceof StorageEvent && e.key !== null && e.key !== KEY) return;
      setDx(readRotorDx());
    };
    window.addEventListener(EVENT, update);
    window.addEventListener('storage', update);
    return () => {
      window.removeEventListener(EVENT, update);
      window.removeEventListener('storage', update);
    };
  }, []);
  return dx;
}
