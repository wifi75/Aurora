import { useQueryClient } from '@tanstack/react-query';
import { useRef, useState } from 'react';
import { ApiError, importLog, newLogInfo, startNewLog, type NewLogInfo } from '../api/client';
import { useT } from '../i18n';
import { NewLogDialog } from './NewLogDialog';

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

/** «Nuovo log vuoto» (Impostazioni › Log, cluster e servizi › Log e servizi): log attuale messo da parte o cancellato, con le conferme. */
export function NewLogBox() {
  const t = useT();
  const client = useQueryClient();
  const [note, setNote] = useState('');
  const [busy, setBusy] = useState(false);
  const [asking, setAsking] = useState<NewLogInfo | null>(null);
  const explain = (error: unknown, generic: string) => {
    if (error instanceof ApiError && error.code === 'local_only') return t('logio.import.local');
    if (error instanceof ApiError && (error.code === 'transmitting' || error.code === 'qso_in_progress')) return t('newlog.busy');
    return error instanceof ApiError ? `${generic} (${error.status} ${error.code})` : generic;
  };
  const askNewLog = async () => {
    setBusy(true);
    try {
      setAsking(await newLogInfo());
      setNote('');
    } catch (error) {
      setNote(explain(error, t('newlog.error')));
    } finally {
      setBusy(false);
    }
  };

  const newLog = async (choice: 'keep' | 'replace' | null) => {
    setAsking(null);
    if (!choice) return;
    setBusy(true);
    try {
      const r = await startNewLog(choice);
      setNote(choice === 'keep' ? t('newlog.doneKeep', { n: r.archived ?? 0, path: r.archive ?? '' }) : t('newlog.doneReplace', { n: r.deleted ?? 0 }));
      // tutto quello che legge il log (lavorati, DXCC, statistiche, mappa, elenco dei QSO) si rilegge dal log nuovo
      await client.invalidateQueries();
    } catch (error) {
      setNote(explain(error, t('newlog.error')));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="log-transfer">
      <p className="muted settings-hint">{t('newlog.intro')}</p>
      <div className="log-transfer-row">
        <button type="button" className="glass log-transfer-button" disabled={busy} title={t('newlog.tip')} onClick={() => void askNewLog()}>
          {t('newlog.button')}
        </button>
      </div>
      {asking ? <NewLogDialog info={asking} onAnswer={(c) => void newLog(c)} /> : null}
      {note ? (
        <p className="live-note" role="status">
          {note}
        </p>
      ) : null}
    </div>
  );
}
