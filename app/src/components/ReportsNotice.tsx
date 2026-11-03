import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { getReports } from '../api/client';
import { useT } from '../i18n';
import { browserStorage } from '../lib/theme';

const SEEN_KEY = 'aurora-reports-notice-seen';

/** Al primo avvio dice una volta che i rapporti degli errori anonimi sono accesi e dove si spengono. */
export function ReportsNotice() {
  const t = useT();
  const [seen, setSeen] = useState(() => {
    try {
      return browserStorage()?.getItem(SEEN_KEY) === '1';
    } catch {
      return false;
    }
  });
  const reports = useQuery({ queryKey: ['reports'], queryFn: getReports, retry: false, enabled: !seen });
  if (seen || !reports.data || (!reports.data.auto_errors && !reports.data.ask_after_crash)) return null;
  const close = () => {
    try {
      browserStorage()?.setItem(SEEN_KEY, '1');
    } catch {
      // senza archivio l'avviso torna al prossimo avvio
    }
    setSeen(true);
  };
  return (
    <div className="reports-notice" role="status">
      <span>{t('rep.notice')}</span>
      <a href="#/impostazioni/support" onClick={close}>
        {t('rep.notice.open')}
      </a>
      <button type="button" className="glass" onClick={close}>
        {t('rep.notice.ok')}
      </button>
    </div>
  );
}
