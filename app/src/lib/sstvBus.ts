import type { AuroraEvent } from '../api/schemas';

/** Un evento SSTV del flusso (righe dell'immagine, stato). */
export type SstvStreamEvent = Extract<AuroraEvent, { type: 'sstv' }>;

type Listener = (event: SstvStreamEvent) => void;

const listeners = new Set<Listener>();

/** Chi disegna l'immagine in arrivo si iscrive qui: le righe non passano dallo stato della Stazione. */
export function onSstv(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function emitSstv(event: SstvStreamEvent): void {
  for (const l of listeners) l(event);
}
