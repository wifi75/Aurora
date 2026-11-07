import '../styles/first-run.css';
import { useEffect } from 'react';
import { createPortal } from 'react-dom';
import { useT, type Key } from '../i18n';

const STEPS = [1, 2, 3, 4, 5, 6] as const;

/** Scheda «L'audio perde blocchi»: cosa controllare sul PC, in ordine. Si apre dall'avviso in basso a destra. */
export function AudioLossHelp({ perMin, onClose }: { perMin: string; onClose: () => void }) {
  const t = useT();
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);
  return createPortal(
    <div className="first-run alh" role="dialog" aria-modal="true" aria-labelledby="alh-title" onClick={onClose}>
      <div className="first-run__card alh-card" onClick={(e) => e.stopPropagation()}>
        <div className="first-run__glow" aria-hidden="true" />
        <header className="alh-head">
          <span className="first-run__brand">Aurora</span>
          <button type="button" className="alh-x" aria-label={t('alh.close')} title={t('alh.close')} onClick={onClose}>
            ×
          </button>
        </header>
        <h2 id="alh-title" className="alh-title">
          {t('alh.title')}
        </h2>
        <p className="alh-lead">{t('alh.lead', { n: perMin })}</p>
        <ol className="alh-steps">
          {STEPS.map((n) => (
            <li key={n}>
              <b>{t(`alh.s${n}.t` as Key)}</b>
              <span>{t(`alh.s${n}.d` as Key)}</span>
            </li>
          ))}
        </ol>
        <p className="alh-foot">{t('alh.foot')}</p>
        <div className="alh-actions">
          <a className="glass" href="#/impostazioni/audio" onClick={onClose}>
            {t('alh.settings')}
          </a>
          <button type="button" className="glass alh-ok" onClick={onClose}>
            {t('alh.close')}
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
