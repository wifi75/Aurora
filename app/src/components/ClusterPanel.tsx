import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { getClusterSpots, getStationInfo, postFrequency, type ClusterSpot } from '../api/client';
import { useT } from '../i18n';
import { filterSpots, spotAge, type SpotFilters } from '../lib/cluster';
import { formatKhz, formatKhzWhole } from '../lib/format';
import { flagCode } from '../lib/flags';
import { bandOf } from '../lib/format';
import { replyErrorMessage } from '../lib/replyError';
import { browserStorage } from '../lib/theme';
import { useStation } from '../state/station';

const KEY = 'aurora-cluster-filters';
const MODES = ['FT8', 'FT4', 'CW', 'SSB', 'RTTY'] as const;

function readFilters(): SpotFilters {
  const defaults: SpotFilters = { band: 'all', mode: 'all', search: '' };
  try {
    const raw = browserStorage()?.getItem(KEY);
    if (!raw) return defaults;
    const p = JSON.parse(raw) as Partial<SpotFilters>;
    return {
      band: typeof p.band === 'string' ? p.band : defaults.band,
      mode: typeof p.mode === 'string' ? p.mode : defaults.mode,
      search: '',
    };
  } catch {
    return defaults;
  }
}

/** Gli spot dei DX Cluster collegati, i piu' recenti per primi; un clic porta la radio sulla banda dello spot. */
export function ClusterPanel() {
  const t = useT();
  const status = useStation((s) => s.status);
  const info = useQuery({ queryKey: ['station-info'], queryFn: getStationInfo, retry: false });
  const cluster = useQuery({ queryKey: ['dx-cluster'], queryFn: () => getClusterSpots(), retry: false, refetchInterval: 4000 });
  const [filters, setFilters] = useState<SpotFilters>(readFilters);
  const [note, setNote] = useState('');

  const myBand = status && status.dial_hz > 0 ? bandOf(status.dial_hz) : null;
  const change = (next: SpotFilters) => {
    setFilters(next);
    try {
      browserStorage()?.setItem(KEY, JSON.stringify({ band: next.band, mode: next.mode }));
    } catch {
      // senza archivio i filtri non si ricordano, ma la finestra funziona lo stesso
    }
  };

  if (cluster.isError) return <p className="muted cluster-note">{t('set.loadError')}</p>;
  if (!cluster.data) return <p className="muted cluster-note">…</p>;
  const { config, servers, spots } = cluster.data;
  if (!config.enabled) {
    return (
      <div className="cluster-note">
        <p className="muted">{t('cluster.off')}</p>
        <a className="glass glass--link" href="#/impostazioni/cluster">
          {t('cluster.settings')}
        </a>
      </div>
    );
  }
  if (config.callsign === '') {
    return (
      <div className="cluster-note">
        <p className="muted">{t('cluster.noCall')}</p>
        <a className="glass glass--link" href="#/impostazioni/cluster">
          {t('cluster.settings')}
        </a>
      </div>
    );
  }

  const linked = servers.filter((s) => s.state === 'connected').length;
  const shown = filterSpots(spots, filters, myBand);

  const tune = async (spot: ClusterSpot) => {
    if (!info.data?.rig_control) {
      setNote(t('cluster.noRig'));
      return;
    }
    try {
      // la frequenza esatta dello spot (kHz -> Hz); la radio non si sposta mai fuori dalle bande dei radioamatori
      const done = await postFrequency(Math.round(spot.freq_khz * 1000));
      if (done.dial_hz) useStation.getState().patchStatus({ dial_hz: done.dial_hz });
      setNote(t('cluster.tuned', { freq: formatKhz(spot.freq_khz) }));
    } catch (error) {
      setNote(replyErrorMessage(error, t));
    }
  };

  return (
    <div className="cluster">
      <div className="cluster-bar" title={t('tip.cluster.filters')}>
        <select aria-label={t('cluster.filter.band')} value={filters.band} onChange={(e) => change({ ...filters, band: e.target.value })}>
          <option value="all">{t('cluster.filter.band')}: {t('cluster.filter.bandAll')}</option>
          <option value="now">{t('cluster.filter.bandNow')}</option>
        </select>
        <select aria-label={t('cluster.filter.mode')} value={filters.mode} onChange={(e) => change({ ...filters, mode: e.target.value })}>
          <option value="all">{t('cluster.filter.mode')}: {t('cluster.filter.modeAll')}</option>
          {MODES.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
        </select>
        <input type="search" aria-label={t('cluster.filter.search')} placeholder={t('cluster.filter.search')} value={filters.search} onChange={(e) => setFilters({ ...filters, search: e.target.value })} />
        <span className={`cluster-link muted${linked === 0 ? ' cluster-link--down' : ''}`}>
          {linked > 0 ? t('cluster.linked', { n: linked, m: servers.length }) : t('cluster.nobody')}
        </span>
      </div>
      {note ? <p className="live-note" role="status">{note}</p> : null}
      {spots.length === 0 ? (
        <p className="muted cluster-note">{t('cluster.waiting')}</p>
      ) : shown.length === 0 ? (
        <p className="muted cluster-note">{t('cluster.none')}</p>
      ) : (
        <table className="cluster-table">
          <thead>
            <tr>
              <th>{t('cluster.col.by')}</th>
              <th>{t('cluster.col.freq')}</th>
              <th>{t('cluster.col.call')}</th>
              <th aria-hidden="true" />
              <th>{t('cluster.col.country')}</th>
              <th>{t('cluster.col.mode')}</th>
              <th>{t('cluster.col.comment')}</th>
              <th>{t('cluster.col.time')}</th>
            </tr>
          </thead>
          <tbody>
            {shown.map((s) => {
              const code = flagCode(s.country);
              const mine = s.band !== null && s.band === myBand;
              return (
                <tr
                  key={s.id}
                  className={`cluster-row${mine ? ' cluster-row--mine' : ''}`}
                  title={t('tip.cluster.row', { spotter: s.spotter, server: s.server, freq: formatKhz(s.freq_khz) })}
                  onDoubleClick={() => void tune(s)}
                >
                  <td className="data cluster-de" title={`${spotAge(s)} s`}>
                    {s.spotter}
                  </td>
                  <td>
                    <span className="cluster-freq data">{formatKhzWhole(s.freq_khz)}</span>
                  </td>
                  <td>
                    <strong className="data cluster-dx">{s.dx_call}</strong>
                  </td>
                  <td className="cluster-flag">{code ? <span className={`fi fi-${code} decode-flag`} aria-hidden="true" /> : null}</td>
                  <td className="cluster-country">{s.country ?? ''}</td>
                  <td className="data cluster-mode">{s.mode ?? ''}</td>
                  <td className="cluster-comment">{s.comment}</td>
                  <td className="data cluster-utc">{s.time_utc.slice(0, 2)}:{s.time_utc.slice(2)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}
