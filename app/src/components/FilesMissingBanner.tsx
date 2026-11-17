import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { getFilesCheck, restoreHamlib } from '../api/client';
import { useT } from '../i18n';
import { browserStorage } from '../lib/theme';

const SEEN_PREFIX = 'aurora-files-missing-seen:';

/** Il file e' uno di quelli di Hamlib (cartella `hamlib`, scritta col percorso di Windows)? */
export function isHamlibFile(name: string): boolean {
  return /^hamlib[\\/]/i.test(name);
}

/** La chiave dell'archivio per un elenco: l'avviso nascosto vale solo per quello, se ne manca un altro ricompare. */
export function seenKey(missing: readonly string[]): string {
  return SEEN_PREFIX + [...missing].map((m) => m.toLowerCase()).sort().join('|');
}

/**
 * Barra ben visibile quando l'antivirus (Windows Defender, Avast e altri) ha tolto dei file ad Aurora: il controllo lo fa il programma
 * all'avvio. Dice quali file, rimanda alla Guida, offre «Ripristina Hamlib» se manca uno dei file di Hamlib e si nasconde con «Ho capito»
 * solo per quell'elenco (se ne manca un altro, o torna a mancare dopo un nuovo avvio con un elenco diverso, ricompare).
 */
export function FilesMissingBanner() {
  const t = useT();
  const queryClient = useQueryClient();
  const check = useQuery({ queryKey: ['files-check'], queryFn: getFilesCheck, retry: false, staleTime: Infinity });
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const [hidden, setHidden] = useState<string | null>(null);
  const missing = check.data?.missing ?? [];
  if (missing.length === 0) return null;
  const key = seenKey(missing);
  let seen = hidden === key;
  if (!seen) {
    try {
      seen = browserStorage()?.getItem(key) === '1';
    } catch {
      seen = false;
    }
  }
  if (seen) return null;
  const files = missing.join(', ');
  const antivirus = (check.data?.antivirus ?? '').trim();
  const close = () => {
    try {
      browserStorage()?.setItem(key, '1');
    } catch {
      // senza archivio l'avviso torna al prossimo avvio
    }
    setHidden(key);
  };
  const restore = () => {
    setBusy(true);
    setFailed(false);
    restoreHamlib()
      .then(() => queryClient.invalidateQueries({ queryKey: ['files-check'] }))
      .catch(() => setFailed(true))
      .finally(() => setBusy(false));
  };
  return (
    <div className="files-missing" role="alert">
      <p className="files-missing-text">{antivirus ? t('files.missing.textNamed', { av: antivirus, files }) : t('files.missing.text', { files })}</p>
      {failed ? <p className="files-missing-fail">{t('files.missing.restoreFail')}</p> : null}
      <div className="files-missing-actions">
        <a className="files-missing-link" href="#/guida/faq-antivirus">
          {t('files.missing.guide')}
        </a>
        {missing.some(isHamlibFile) ? (
          <button type="button" className="files-missing-restore" disabled={busy} onClick={restore}>
            {busy ? t('cat.restore.busy') : t('cat.restore')}
          </button>
        ) : null}
        <button type="button" className="files-missing-ok" onClick={close}>
          {t('files.missing.dismiss')}
        </button>
      </div>
    </div>
  );
}
