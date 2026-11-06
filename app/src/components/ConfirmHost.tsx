import { useEffect, useRef } from 'react';
import { createRoot } from 'react-dom/client';
import { useT } from '../i18n';

/**
 * Chiede una conferma con una finestra nella grafica di Aurora (al posto di `window.confirm`, che apre la finestra grigia del browser).
 * Risolve `true` con «OK», `false` con «Annulla», Esc o clic fuori. La finestra si crea da sola sopra a tutto (anche sopra le
 * Impostazioni) e si toglie alla risposta: non serve averla montata da nessuna parte.
 */
export function askConfirm(text: string): Promise<boolean> {
  return new Promise((resolve) => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    const answer = (ok: boolean) => {
      root.unmount();
      host.remove();
      resolve(ok);
    };
    root.render(<ConfirmDialog text={text} onAnswer={answer} />);
  });
}

function ConfirmDialog({ text, onAnswer }: { text: string; onAnswer(ok: boolean): void }) {
  const t = useT();
  const ok = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    ok.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onAnswer(false);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onAnswer]);
  return (
    <div className="confirm-overlay" role="presentation" onMouseDown={(e) => e.target === e.currentTarget && onAnswer(false)}>
      <div className="confirm-card" role="alertdialog" aria-modal="true" aria-labelledby="confirm-text">
        <p id="confirm-text" className="confirm-text">
          {text}
        </p>
        <div className="confirm-actions">
          <button type="button" className="confirm-btn" onClick={() => onAnswer(false)}>
            {t('confirm.cancel')}
          </button>
          <button ref={ok} type="button" className="confirm-btn confirm-btn--ok" onClick={() => onAnswer(true)}>
            {t('confirm.ok')}
          </button>
        </div>
      </div>
    </div>
  );
}

/** Rimasta per compatibilita': la conferma ora si crea da sola, qui non serve montare niente. */
export function ConfirmHost() {
  return null;
}
