import { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import type { NewLogInfo } from '../api/client';
import { useT } from '../i18n';

/**
 * La conferma di «Nuovo log vuoto»: un avviso ben visibile con due scelte, «tieni il log attuale» (consigliata, lo mette da parte con data e ora)
 * e «cancella il log attuale» (in rosso, con una seconda conferma). Annulla e' la scelta predefinita (Invio ed Esc non fanno partire niente; per cancellare va scritta una parola); c'e' sempre l'invito a esportare prima l'ADIF.
 */
export function NewLogDialog({ info, onAnswer }: { info: NewLogInfo; onAnswer(choice: 'keep' | 'replace' | null): void }) {
  const t = useT();
  const [danger, setDanger] = useState(false);
  const [typed, setTyped] = useState('');
  const word = t('newlog.word');
  const cancel = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    cancel.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onAnswer(null);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onAnswer, danger]);
  const exportFirst = (
    <a className="confirm-btn newlog-export" href="#/esporta" onClick={() => onAnswer(null)}>
      {t('newlog.exportFirst')}
    </a>
  );
  return createPortal(
    <div className="confirm-overlay" role="presentation" onMouseDown={(e) => e.target === e.currentTarget && onAnswer(null)}>
      <div className={`confirm-card newlog-card${danger ? ' newlog-card--danger' : ''}`} role="alertdialog" aria-modal="true" aria-labelledby="newlog-title" aria-describedby="newlog-text">
        <p id="newlog-title" className="newlog-title">
          {danger ? t('newlog.dangerTitle') : t('newlog.title')}
        </p>
        {danger ? (
          <div id="newlog-text" className="confirm-text">
            <p className="newlog-danger-text">{t('newlog.dangerText', { n: info.qsos })}</p>
            <label className="newlog-type">
              {t('newlog.typeWord', { word })}
              <input value={typed} onChange={(e) => setTyped(e.target.value)} autoComplete="off" spellCheck={false} />
            </label>
          </div>
        ) : (
          <div id="newlog-text" className="confirm-text">
            <p>{t('newlog.count', { n: info.qsos, file: info.file })}</p>
            <p>{t('newlog.exportHint')}</p>
          </div>
        )}
        <div className="newlog-actions">
          {exportFirst}
          {danger ? (
            <button type="button" className="confirm-btn newlog-btn--danger" disabled={typed.trim().toUpperCase() !== word.toUpperCase()} onClick={() => onAnswer('replace')}>
              {t('newlog.replaceConfirm', { n: info.qsos })}
            </button>
          ) : (
            <>
              <button type="button" className="confirm-btn confirm-btn--ok newlog-choice" onClick={() => onAnswer('keep')}>
                <strong>{t('newlog.keep')}</strong>
                <span>{t('newlog.keepHint', { folder: info.folder })}</span>
              </button>
              <button type="button" className="confirm-btn newlog-choice newlog-btn--danger" onClick={() => setDanger(true)}>
                <strong>{t('newlog.replace')}</strong>
                <span>{t('newlog.replaceHint')}</span>
              </button>
            </>
          )}
          <button ref={cancel} type="button" className="confirm-btn" onClick={() => onAnswer(null)}>
            {t('confirm.cancel')}
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
