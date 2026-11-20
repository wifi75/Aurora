import { useEffect, useRef, useState } from 'react';
import { useT } from '../i18n';
import type { ImportHow, ImportReport } from '../lib/fldigiMacros';

interface Props {
  report: ImportReport;
  /** Il modo in uso (PSK31, RTTY...), per la scelta «solo per questo modo». */
  modeLabel: string;
  onConfirm(where: 'all' | 'mode', how: ImportHow): void;
  onCancel(): void;
}

/**
 * «Importa le macro di fldigi»: dopo aver letto il file si dice cosa c'e' dentro (quante macro, quali tag non si traducono) e si sceglie dove metterle
 * (per tutti i modi o solo per quello in uso) e come (solo i tasti del file, oppure il gruppo diventa quello del file). Niente cambia finche' non si preme «Importa».
 */
export function FldigiImportDialog({ report, modeLabel, onConfirm, onCancel }: Props) {
  const t = useT();
  const [where, setWhere] = useState<'all' | 'mode'>('all');
  const [how, setHow] = useState<ImportHow>('add');
  const ok = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    ok.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onCancel();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onCancel]);
  return (
    <div className="confirm-overlay" role="presentation" onMouseDown={(e) => e.target === e.currentTarget && onCancel()}>
      <div className="confirm-card fldigi-card" role="dialog" aria-modal="true" aria-labelledby="fldigi-title">
        <h3 id="fldigi-title" className="fldigi-title">
          {t('fldigi.title')}
        </h3>
        <p className="confirm-text">{t('fldigi.found', { n: String(report.macros.length) })}</p>
        {report.untranslated.length > 0 ? <p className="confirm-text fldigi-warn">{t('fldigi.untranslated', { tags: report.untranslated.join(' ') })}</p> : null}
        <fieldset className="fldigi-opts">
          <legend>{t('fldigi.where')}</legend>
          <label>
            <input type="radio" name="fldigi-where" checked={where === 'all'} onChange={() => setWhere('all')} /> {t('fldigi.where.all')}
          </label>
          <label>
            <input type="radio" name="fldigi-where" checked={where === 'mode'} onChange={() => setWhere('mode')} /> {t('fldigi.where.mode', { mode: modeLabel })}
          </label>
        </fieldset>
        <fieldset className="fldigi-opts">
          <legend>{t('fldigi.how')}</legend>
          <label>
            <input type="radio" name="fldigi-how" checked={how === 'add'} onChange={() => setHow('add')} /> {t('fldigi.how.add')}
          </label>
          <label>
            <input type="radio" name="fldigi-how" checked={how === 'replace'} onChange={() => setHow('replace')} /> {t('fldigi.how.replace')}
          </label>
        </fieldset>
        <div className="confirm-actions">
          <button type="button" className="confirm-btn" onClick={onCancel}>
            {t('confirm.cancel')}
          </button>
          <button ref={ok} type="button" className="confirm-btn confirm-btn--ok" onClick={() => onConfirm(where, how)}>
            {t('fldigi.import')}
          </button>
        </div>
      </div>
    </div>
  );
}
