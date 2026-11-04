import { useQueryClient } from '@tanstack/react-query';
import { useRef, useState } from 'react';
import { ApiError, importLog } from '../api/client';
import { useT } from '../i18n';

/** Porta alla pagina di esportazione del log interno (`#/esporta`) e ci importa un file ADIF senza doppioni. */
export function LogTransfer() {
  const t = useT();
  const client = useQueryClient();
  const [note, setNote] = useState('');
  const [busy, setBusy] = useState(false);
  const input = useRef<HTMLInputElement | null>(null);

  const pick = async (file: File | undefined) => {
    if (!file) return;
    setBusy(true);
    setNote(t('logio.import.busy'));
    try {
      const r = await importLog(file);
      setNote(t('logio.import.done', { added: r.added, duplicates: r.duplicates, skipped: r.skipped, read: r.read }));
      // il log e' cambiato: scheda DXCC e statistiche si rileggono
      await client.invalidateQueries({ queryKey: ['dxcc-matrix'] });
      await client.invalidateQueries({ queryKey: ['stats'] });
    } catch (error) {
      if (error instanceof ApiError && error.code === 'local_only') setNote(t('logio.import.local'));
      else if (error instanceof ApiError && error.code === 'empty_file') setNote(t('logio.import.empty'));
      else if (error instanceof ApiError && error.status === 413) setNote(t('logio.import.tooBig'));
      else setNote(error instanceof ApiError ? `${t('logio.import.error')} (${error.status} ${error.code})` : t('logio.import.error'));
    } finally {
      setBusy(false);
      if (input.current) input.current.value = '';
    }
  };

  return (
    <div className="log-transfer">
      <p className="muted settings-hint">{t('logio.intro')}</p>
      <div className="log-transfer-row">
        <a className="glass log-transfer-button" href="#/esporta" title={t('logio.export.tip')}>
          {t('logio.export')}
        </a>
      </div>
      <div className="log-transfer-row">
        <button type="button" className="glass log-transfer-button" disabled={busy} title={t('logio.import.tip')} onClick={() => input.current?.click()}>
          {t('logio.import')}
        </button>
        <input ref={input} type="file" accept=".adi,.adif,.txt" className="visually-hidden" tabIndex={-1} aria-label={t('logio.import')} onChange={(e) => void pick(e.target.files?.[0])} />
      </div>
      {note ? (
        <p className="live-note" role="status">
          {note}
        </p>
      ) : null}
    </div>
  );
}
