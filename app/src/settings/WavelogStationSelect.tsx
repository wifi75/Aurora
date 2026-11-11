import { Icon } from '../components/Icon';
import { useQuery } from '@tanstack/react-query';
import { postWavelogStations } from '../api/client';
import { useT } from '../i18n';

/**
 * La stazione di Wavelog/Cloudlog scelta da una tendina con le stazioni lette dal server (indirizzo scritto qui e chiave salvata).
 * Se il server non risponde resta la casella del numero, con il motivo accanto.
 */
export function WavelogStationSelect({ id, label, url, value, disabled, onChange }: { id: string; label: string; url: string; value: string; disabled: boolean; onChange(v: string): void }) {
  const t = useT();
  const query = useQuery({
    queryKey: ['wavelog-stations', url.trim()],
    queryFn: () => postWavelogStations(url.trim()),
    enabled: !disabled && url.trim() !== '',
    retry: false,
    staleTime: 60_000,
  });
  const stations = query.data ?? [];
  if (stations.length === 0) {
    return (
      <span className="wl-station">
        <input id={id} aria-label={label} type="text" className="jx-text" disabled={disabled} maxLength={200} value={value} autoComplete="off" onChange={(e) => onChange(e.target.value)} />
        {query.isError ? <small className="ens-reason muted">{t('ens.wl.noList')}</small> : null}
        {!disabled && url.trim() !== '' ? (
          <button title={t('tip.ui.wavelogRefresh')} type="button" className="glass" onClick={() => void query.refetch()} disabled={query.isFetching}>
            {query.isFetching ? t('ens.wl.reading') : t('ens.wl.read')}
          </button>
        ) : null}
      </span>
    );
  }
  const known = stations.some((s) => s.id === value);
  return (
    <span className="wl-station">
      <select id={id} aria-label={label} disabled={disabled} value={value} onChange={(e) => onChange(e.target.value)}>
        {!known ? <option value={value}>{value === '' ? t('ens.wl.pick') : `${value} — ${t('ens.wl.unknown')}`}</option> : null}
        {stations.map((s) => (
          <option key={s.id} value={s.id}>
            {`${s.id} — ${s.name}${s.call ? ` (${s.call})` : ''}`}
          </option>
        ))}
      </select>
      <button type="button" className="glass icon-btn" title={t('ens.wl.read')} aria-label={t('ens.wl.read')} onClick={() => void query.refetch()} disabled={query.isFetching}>
        <Icon name="refresh" />
      </button>
    </span>
  );
}
