import { useEffect, useState } from 'react';
import { browserStorage } from './theme';

/** Colore di evidenziazione delle DXpedition nel DX Cluster: lo sceglie l'operatore (Impostazioni > DX Cluster), di base giallo fluo. */
const KEY = 'aurora-cluster-dxped-color';
export const DXPED_DEFAULT = '#e6ff00';

/** Tavolozza pronta: sei colori fluo (giallo, verde, arancio, rosa, azzurro, viola) e due tenui. */
export const DXPED_PALETTE: ReadonlyArray<{ color: string; fluo: boolean }> = [
  { color: '#e6ff00', fluo: true },
  { color: '#39ff14', fluo: true },
  { color: '#ff9f1a', fluo: true },
  { color: '#ff2fa8', fluo: true },
  { color: '#00e5ff', fluo: true },
  { color: '#c04dff', fluo: true },
  { color: '#ffd23f', fluo: false },
  { color: '#8fd3ff', fluo: false },
];

const HEX = /^#[0-9a-f]{6}$/i;
const listeners = new Set<(c: string) => void>();

export function readDxpedColor(): string {
  try {
    const v = browserStorage()?.getItem(KEY) ?? '';
    return HEX.test(v) ? v.toLowerCase() : DXPED_DEFAULT;
  } catch {
    return DXPED_DEFAULT;
  }
}

export function setDxpedColor(color: string): void {
  const c = HEX.test(color) ? color.toLowerCase() : DXPED_DEFAULT;
  try {
    browserStorage()?.setItem(KEY, c);
  } catch {
    // senza archivio del browser vale solo per questa sessione
  }
  listeners.forEach((l) => l(c));
}

/** Testo leggibile sopra il colore: scuro sui colori chiari, chiaro sugli scuri. */
export function inkFor(color: string): string {
  const n = parseInt(color.slice(1), 16);
  const r = (n >> 16) & 255;
  const g = (n >> 8) & 255;
  const b = n & 255;
  return 0.299 * r + 0.587 * g + 0.114 * b > 150 ? '#141400' : '#ffffff';
}

export function useDxpedColor(): string {
  const [color, setColor] = useState(readDxpedColor);
  useEffect(() => {
    listeners.add(setColor);
    return () => {
      listeners.delete(setColor);
    };
  }, []);
  return color;
}
