import { useT } from '../i18n';
import { useToasts } from '../lib/toast';

const ICON = { ok: '✓', error: '!', info: 'i' } as const;

/**
 * Le notifiche in alto al centro: vetro traslucido colorato per tipo (riuscito, errore, informazione), entrano scivolando, una barretta
 * mostra quanto resta e spariscono sfumando verso l'alto. Un clic le chiude subito.
 */
export function Toaster() {
  const t = useT();
  const toasts = useToasts((s) => s.toasts);
  const dismiss = useToasts((s) => s.dismiss);
  return (
    <div className="toaster" aria-live="polite" aria-relevant="additions">
      {toasts.map((x) => (
        <div
          key={x.id}
          className={`toast toast--${x.kind}${x.leaving ? ' toast--leaving' : ''}`}
          role={x.kind === 'error' ? 'alert' : 'status'}
          style={{ '--toast-ms': `${x.ms}ms` } as React.CSSProperties}
          onClick={() => dismiss(x.id)}
          title={t('toast.close')}
        >
          <span className="toast-icon" aria-hidden="true">
            {ICON[x.kind]}
          </span>
          <span className="toast-text">{x.text}</span>
          <span className="toast-bar" aria-hidden="true" />
        </div>
      ))}
    </div>
  );
}
