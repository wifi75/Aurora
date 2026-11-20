import type { AuroraEvent } from '../api/schemas';

/** Un evento WEFAX del flusso (righe della cartina, stato). */
export type WefaxStreamEvent = Extract<AuroraEvent, { type: 'wefax' }>;

type Listener = (event: WefaxStreamEvent) => void;

const listeners = new Set<Listener>();

/** Chi disegna la cartina in arrivo si iscrive qui: le righe non passano dallo stato della Stazione. */
export function onWefax(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function emitWefax(event: WefaxStreamEvent): void {
  for (const l of listeners) l(event);
}
