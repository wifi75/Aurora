import { useState } from 'react';
import { ApiError, postHalt } from '../api/client';

type Phase = { kind: 'idle' } | { kind: 'ok' } | { kind: 'error'; message: string };

function messageFor(error: unknown): string {
  if (error instanceof ApiError && error.status === 401) {
    return 'Dispositivo non associato: associalo di nuovo con un codice dal PC.';
  }
  return 'Non sono riuscito a fermare la trasmissione. Controlla la connessione e riprova.';
}

interface Props {
  /** `bar`: barra fissa in fondo (telefono). `compact`: pulsante grande nella barra in alto (PC). */
  variant?: 'bar' | 'compact';
}

/** Sempre attivo, anche con il collegamento caduto: un tocco, nessuna conferma. */
export function StopButton({ variant = 'bar' }: Props) {
  const [phase, setPhase] = useState<Phase>({ kind: 'idle' });

  const stop = async () => {
    try {
      await postHalt();
      setPhase({ kind: 'ok' });
    } catch (error) {
      setPhase({ kind: 'error', message: messageFor(error) });
    }
  };

  return (
    <div className={variant === 'bar' ? 'stop-bar' : 'stop-compact'}>
      <button type="button" className="stop-btn" onClick={stop}>
        STOP TX
      </button>
      <p className="stop-note" role="status" aria-live="polite">
        {phase.kind === 'ok' ? 'Trasmissione fermata' : phase.kind === 'error' ? phase.message : ''}
      </p>
    </div>
  );
}
