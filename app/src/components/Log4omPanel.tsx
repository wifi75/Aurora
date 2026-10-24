import { useQuery } from '@tanstack/react-query';
import { getLog4om } from '../api/client';
import { useT } from '../i18n';
import { usePrefs } from '../lib/prefs';

/** Gli ultimi QSO del log di Log4OM2 (sola lettura): quanti, si sceglie in Impostazioni → Generali. */
export function Log4omPanel() {
  const t = useT();
  const count = usePrefs((s) => s.prefs.log4omCount);
  const log = useQuery({ queryKey: ['log4om', count], queryFn: () => getLog4om(count), refetchInterval: 30_000, retry: false });

  if (log.isError) return <p className="muted log4om-note">{t('log.unavailable')}</p>;
  if (!log.data) return <p className="muted log4om-note">…</p>;
  if (log.data.qsos.length === 0) return <p className="muted log4om-note">{t('log.empty')}</p>;
  return (
    <div className="log4om">
      <p className="muted log4om-total">{t('log.total', { n: log.data.total })}</p>
      <table className="log4om-table">
        <thead>
          <tr>
            <th>{t('log.col.date')}</th>
            <th>{t('log.col.call')}</th>
            <th>{t('log.col.band')}</th>
            <th>{t('log.col.mode')}</th>
            <th>{t('log.col.rst')}</th>
            <th>{t('log.col.country')}</th>
          </tr>
        </thead>
        <tbody>
          {log.data.qsos.map((q, i) => (
            <tr key={`${q.date}-${q.call}-${i}`}>
              <td className="data">{q.date.replace('Z', '').slice(0, 16)}</td>
              <td className="data log4om-call">{q.call}</td>
              <td className="data">{q.band}</td>
              <td className="data">{q.mode}</td>
              <td className="data">{[q.rst_sent, q.rst_rcvd].filter(Boolean).join(' / ')}</td>
              <td>{q.country}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
