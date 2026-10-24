import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
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

/** Barra in alto quando sul repository e' uscita una versione di Aurora piu' nuova di quella in uso; «Piu' tardi» la nasconde fino alla versione dopo. */
export function UpdateBar() {
  const t = useT();
  const update = useQuery({ queryKey: ['update'], queryFn: getUpdate, retry: false, refetchInterval: 10 * 60_000 });
  const [hidden, setHidden] = useState(dismissedVersion);
  const notice = useStation((s) => s.updateNotice);
  const [installing, setInstalling] = useState(false);
  const [failure, setFailure] = useState('');
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
  if (!release || hidden === release.version) return null;

  return (
    <div className="update-bar" role="status">
      <strong>{t('upd.banner', { version: release.version, current: update.data?.current ?? '…' })}</strong>
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
  );
}
