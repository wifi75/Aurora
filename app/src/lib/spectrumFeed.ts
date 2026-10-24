/** Canale leggero tra il flusso del core e la cascata: le righe non passano da React. */
type Listener = (bins: Uint8Array) => void;

const listeners = new Set<Listener>();

export function registerSpectrum(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function emitSpectrum(bins: Uint8Array): void {
  for (const listener of listeners) listener(bins);
}
