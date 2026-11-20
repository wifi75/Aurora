import type { StationStatus } from '../api/schemas';
import { browserStorage } from './theme';

/** Chiave dell'interruttore «Aggiorna da solo quando Aurora e' ferma» (preferenza di questo PC, spenta di base). */
export const UPDATE_WHEN_IDLE_KEY = 'aurora-update-when-idle';

/** Da quanto tempo nessuno tocca mouse o tastiera perche' Aurora si consideri ferma. */
export const IDLE_AFTER_MS = 10 * 60_000;

export function getUpdateWhenIdle(): boolean {
  try {
    return browserStorage()?.getItem(UPDATE_WHEN_IDLE_KEY) === '1';
  } catch {
    return false;
  }
}

export function setUpdateWhenIdle(on: boolean): void {
  try {
    browserStorage()?.setItem(UPDATE_WHEN_IDLE_KEY, on ? '1' : '0');
  } catch {
    // senza archivio l'interruttore torna spento alla prossima apertura
  }
}

/**
 * Aurora e' ferma quando non si sta trasmettendo, la trasmissione non e' abilitata, non c'e' un QSO ne' una stazione in coda
 * e nessuno ha toccato mouse o tastiera da `IDLE_AFTER_MS`. Senza lo stato della stazione (non ancora arrivato) non e' ferma.
 */
export function isStationIdle(status: StationStatus | null | undefined, idleForMs: number): boolean {
  if (!status) return false;
  if (status.transmitting || status.tx_enabled) return false;
  if ((status.dx_call ?? '') !== '' || (status.queue?.length ?? 0) > 0) return false;
  return idleForMs >= IDLE_AFTER_MS;
}
