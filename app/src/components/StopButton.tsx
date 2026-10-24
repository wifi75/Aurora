import { useState } from 'react';
import { ApiError, postHalt } from '../api/client';
import { useT, type T } from '../i18n';
import { useStation } from '../state/station';

type Phase = { kind: 'idle' } | { kind: 'ok' } | { kind: 'error'; message: string };

function messageFor(error: unknown, t: T): string {
  if (error instanceof ApiError && error.status === 401) return t('stop.unpaired');
  return t('stop.fail');
}

interface Props {
  /** `bar`: barra fissa in fondo (telefono). `compact`: pulsante grande nella barra in alto (PC). */
  variant?: 'bar' | 'compact';
}

/** Sempre attivo, anche con il collegamento caduto: un tocco, nessuna conferma. */
export function StopButton({ variant = 'bar' }: Props) {
  const t = useT();
  // in trasmissione e' il pulsante pieno e ben visibile; a riposo resta una presenza discreta (sempre attivo)
  const transmitting = useStation((s) => s.status?.transmitting ?? false);
  const [phase, setPhase] = useState<Phase>({ kind: 'idle' });

  const stop = async () => {
    try {
      await postHalt();
      setPhase({ kind: 'ok' });
    } catch (error) {
      setPhase({ kind: 'error', message: messageFor(error, t) });
    }
  };

  return (
    <div className={variant === 'bar' ? 'stop-bar' : 'stop-compact'}>
      <button type="button" className={`stop-btn${transmitting ? '' : ' stop-btn--idle'}`} onClick={stop} title={t('tip.stop')}>
        {t('stop.button')}
      </button>
      <p className="stop-note" role="status" aria-live="polite">
        {phase.kind === 'ok' ? t('stop.ok') : phase.kind === 'error' ? phase.message : ''}
      </p>
    </div>
  );
}
