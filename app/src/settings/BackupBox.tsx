import { useRef, useState } from 'react';
import { ApiError, makeSettingsBackup, restoreSettingsBackup } from '../api/client';
import { useT } from '../i18n';

/**
 * Riquadro «Copia delle impostazioni»: uno zip sul Desktop con impostazioni e log dei QSO, e il ripristino da uno zip fatto cosi'
 * (si applica al prossimo avvio di Aurora). Password e chiavi dei servizi non entrano nella copia.
 */
export function BackupBox({ available }: { available: boolean }) {
  const t = useT();
  const file = useRef<HTMLInputElement>(null);
  const [busy, setBusy] = useState<'make' | 'restore' | null>(null);
  const [done, setDone] = useState('');
  const [failure, setFailure] = useState('');

  const make = async () => {
    setBusy('make');
    setFailure('');
    setDone('');
    try {
      const r = await makeSettingsBackup();
      setDone(t('bak.made', { name: r.name, files: r.files }));
    } catch (error) {
      setFailure(t('bak.failed', { message: error instanceof ApiError ? (error.detail ?? error.code) : '—' }));
    } finally {
      setBusy(null);
    }
  };

  const restore = async (picked: File | undefined) => {
    if (!picked) return;
    if (!window.confirm(t('bak.confirm'))) return;
    setBusy('restore');
    setFailure('');
    setDone('');
    try {
      const r = await restoreSettingsBackup(picked);
      setDone(t('bak.restored', { files: r.files }));
    } catch (error) {
      setFailure(error instanceof ApiError && error.code === 'not_a_backup' ? t('bak.notBackup') : t('bak.failed', { message: error instanceof ApiError ? (error.detail ?? error.code) : '—' }));
    } finally {
      setBusy(null);
      if (file.current) file.current.value = '';
    }
  };

  return (
    <section className="about-fact about-support">
      <h3>{t('bak.title')}</h3>
      <p>{t('bak.intro')}</p>
      {available ? (
        <div className="about-support-actions">
          <button type="button" className="glass about-support-main" onClick={() => void make()} disabled={busy !== null}>
            {busy === 'make' ? t('bak.working') : t('bak.make')}
          </button>
          <button type="button" className="glass" onClick={() => file.current?.click()} disabled={busy !== null}>
            {busy === 'restore' ? t('bak.working') : t('bak.restore')}
          </button>
          <input ref={file} type="file" accept=".zip,application/zip" hidden onChange={(e) => void restore(e.target.files?.[0])} />
        </div>
      ) : (
        <p className="about-support-note">{t('sup.onlyPc')}</p>
      )}
      {done ? <p className="about-support-ready" role="status">{done}</p> : null}
      {failure ? <p className="about-support-alert" role="alert">{failure}</p> : null}
      <p className="about-support-note">{t('bak.note')}</p>
    </section>
  );
}
