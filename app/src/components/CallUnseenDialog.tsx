import { useEffect, useRef } from 'react';
import { createRoot } from 'react-dom/client';
import { useT } from '../i18n';
import { callConfirmText } from '../lib/seenStation';

/** «Vuoi chiamare XX9BB (Macao)?»: risolve `true` con Chiama o Invio, `false` con Annulla, Esc o clic fuori. */
export function askCallUnseen(call: string, country?: string | null): Promise<boolean> {
  return new Promise((resolve) => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    let done = false;
    const answer = (ok: boolean) => {
      if (done) return;
      done = true;
      root.unmount();
      host.remove();
      resolve(ok);
    };
    root.render(<CallUnseenDialog call={call} country={country} onAnswer={answer} />);
  });
}

function CallUnseenDialog({ call, country, onAnswer }: { call: string; country?: string | null; onAnswer(ok: boolean): void }) {
  const t = useT();
  const ok = useRef<HTMLButtonElement | null>(null);
  const cancel = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    ok.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onAnswer(false);
      else if (e.key === 'Tab') {
        // focus trap leggero: Tab alterna tra i due pulsanti
        e.preventDefault();
        (document.activeElement === ok.current ? cancel.current : ok.current)?.focus();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onAnswer]);
  return (
    <div className="confirm-overlay" role="presentation" onMouseDown={(e) => e.target === e.currentTarget && onAnswer(false)}>
      <div className="confirm-card" role="alertdialog" aria-modal="true" aria-labelledby="callunseen-text">
        <p id="callunseen-text" className="confirm-text">
          {callConfirmText(t('callUnseen.text'), call, country)}
        </p>
        <div className="confirm-actions">
          <button ref={cancel} type="button" className="confirm-btn" onClick={() => onAnswer(false)}>
            {t('confirm.cancel')}
          </button>
          <button ref={ok} type="button" className="confirm-btn confirm-btn--ok" onClick={() => onAnswer(true)}>
            {t('callUnseen.call')}
          </button>
        </div>
      </div>
    </div>
  );
}
