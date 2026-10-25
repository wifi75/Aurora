import { useQuery } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import { ApiError, getUpdate, installUpdate } from '../api/client';
import { useT } from '../i18n';
import { browserStorage } from '../lib/theme';
import { useStation } from '../state/station';

const KEY = 'aurora-update-dismissed';

function dismissedVersion(): string {
  try {
    return browserStorage()?.getItem(KEY) ?? '';
  } catch {
    return '';
  }
}

/**
 * Quando sul repository e' uscita una versione di Aurora piu' nuova di quella in uso, in testata compare una piccola icona (prima della lingua);
 * un clic apre le azioni: Installa ora, Note e download, Piu' tardi (nasconde l'avviso fino alla versione dopo).
 */
export function UpdateBar() {
  const t = useT();
  const update = useQuery({ queryKey: ['update'], queryFn: getUpdate, retry: false, refetchInterval: 10 * 60_000 });
  const [hidden, setHidden] = useState(dismissedVersion);
  const notice = useStation((s) => s.updateNotice);
  const [installing, setInstalling] = useState(false);
  const [failure, setFailure] = useState('');
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement | null>(null);
  const install = async () => {
    setInstalling(true);
    setFailure('');
    try {
      await installUpdate();
    } catch (error) {
      setInstalling(false);
      setFailure(t('upd.installFailed', { message: error instanceof ApiError ? error.code : '—' }));
    }
  };
  // quando il core avvisa di una versione nuova i dati della pagina sono vecchi (senza «Installa ora»): si rileggono
  const refetch = update.refetch;
  useEffect(() => {
    if (notice) void refetch();
  }, [notice, refetch]);
  // l'evento del core arriva subito; il controllo della pagina dopo, con tutti i dettagli
  const release = update.data?.available ?? (notice ? { version: notice.version, tag: notice.tag, name: '', url: notice.url } : null);
  // il pannello si chiude con Esc o con un clic fuori
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    const onDown = (e: MouseEvent) => {
      if (box.current && !box.current.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener('keydown', onKey);
    window.addEventListener('mousedown', onDown);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('mousedown', onDown);
    };
  }, [open]);
  if (!release || hidden === release.version) return null;
  const banner = t('upd.banner', { version: release.version, current: update.data?.current ?? '…' });

  return (
    <div className="update-badge" ref={box}>
      <button type="button" className="update-icon" aria-label={banner} aria-expanded={open} aria-haspopup="true" title={banner} onClick={() => setOpen((v) => !v)}>
        <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M12 3v12" />
          <path d="m7 11 5 5 5-5" />
          <path d="M5 20h14" />
        </svg>
        <span className="update-dot" aria-hidden="true" />
      </button>
      {open ? (
    <div className="update-pop" role="status">
      <strong>{banner}</strong>
      {release.name && release.name !== release.tag ? <span className="muted">{release.name}</span> : null}
      {update.data?.can_install ? (
        <button type="button" className="glass glass--on" disabled={installing} onClick={() => void install()}>
          {t('upd.install')}
        </button>
      ) : null}
      {installing ? <span className="muted">{t('upd.installing')}</span> : null}
      {failure ? <span role="alert">{failure}</span> : null}
      {release.url ? (
        <a className="glass glass--link" href={release.url} target="_blank" rel="noreferrer noopener">
          {t('upd.open')}
        </a>
      ) : null}
      <button
        type="button"
        className="glass"
        onClick={() => {
          try {
            browserStorage()?.setItem(KEY, release.version);
          } catch {
            // senza archivio la barra torna alla prossima apertura
          }
          setHidden(release.version);
        }}
      >
        {t('upd.later')}
      </button>
    </div>
      ) : null}
    </div>
  );
}
