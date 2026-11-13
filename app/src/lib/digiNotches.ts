import type { DigiNotch } from '../api/client';

/** Larghezza di un notch: di base e limiti (Hz), come nel motore. */
export const NOTCH_WIDTH_DEFAULT = 30;
export const NOTCH_WIDTH_MIN = 10;
export const NOTCH_WIDTH_MAX = 100;
const HZ_MIN = 50;
const HZ_MAX = 5950;

const clampHz = (hz: number) => Math.round(Math.min(HZ_MAX, Math.max(HZ_MIN, hz)) * 10) / 10;
const clampWidth = (w: number) => Math.round(Math.min(NOTCH_WIDTH_MAX, Math.max(NOTCH_WIDTH_MIN, w)));

/** Un notch nuovo a `hz` con la larghezza di base; se cade dentro uno che c'e' gia' non se ne aggiunge un altro. */
export function addNotch(list: DigiNotch[], hz: number, width = NOTCH_WIDTH_DEFAULT): DigiNotch[] {
  if (notchAt(list, hz) !== -1) return list;
  return [...list, { hz: clampHz(hz), width_hz: clampWidth(width) }];
}

/** L'indice del notch che copre `hz` (con `slackHz` di tolleranza), -1 se nessuno. */
export function notchAt(list: DigiNotch[], hz: number, slackHz = 0): number {
  return list.findIndex((n) => Math.abs(n.hz - hz) <= n.width_hz / 2 + slackHz);
}

export function removeNotch(list: DigiNotch[], index: number): DigiNotch[] {
  return list.filter((_, i) => i !== index);
}

export function moveNotch(list: DigiNotch[], index: number, hz: number): DigiNotch[] {
  return list.map((n, i) => (i === index ? { ...n, hz: clampHz(hz) } : n));
}

/** La larghezza da un bordo trascinato fino a `edgeHz`: il centro resta fermo. */
export function resizeNotch(list: DigiNotch[], index: number, edgeHz: number): DigiNotch[] {
  return list.map((n, i) => (i === index ? { ...n, width_hz: clampWidth(2 * Math.abs(edgeHz - n.hz)) } : n));
}
