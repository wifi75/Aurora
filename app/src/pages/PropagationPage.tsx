import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import { getOnline, getPsk, getSolar, refreshOnline, type SolarData } from '../api/client';
import { Gauge } from '../components/Gauge';
import { WorldMap, type MapReceiver } from '../components/WorldMap';
import { useT, type Key } from '../i18n';
import { bandOf, formatClock } from '../lib/format';
import { flagCode } from '../lib/flags';
import { gridCenter } from '../lib/geo';
import { BAND_LEGEND, bandColor } from '../lib/mapColors';
import { readMapOptions } from '../lib/mapOptions';
import { formatDistance, usePrefs } from '../lib/prefs';
import { bandCounts, bestSnr, conditionGroup, latestPerReceiver, pskRows, summarize, type PskRow } from '../lib/psk';
import { aLevel, conditionLevel, kLevel, sfiLevel, type Level } from '../lib/solar';
import { isNight } from '../lib/terminator';
import { useStation } from '../state/station';
import { mapLabels } from './MapPage';

/** Divide le righe in blocchi, per affiancare piu' colonne di tabella e vedere piu' stazioni senza scorrere. */
function chunk<T>(items: readonly T[], size: number): T[][] {
  const out: T[][] = [];
  for (let i = 0; i < items.length; i += size) out.push(items.slice(i, i + size));
  return out;
}

const GROUPS = ['80m-40m', '30m-20m', '17m-15m', '12m-10m'] as const;

/** Le tre barrette di una condizione: una scarsa, due discreta, tre buona. */
function Meter({ level }: { level: Level }) {
  const filled = level === 'good' ? 3 : level === 'fair' ? 2 : level === 'poor' ? 1 : 0;
  return (
    <span className="prop-meter" aria-hidden="true">
      {[1, 2, 3].map((n) => (
        <i key={n} className={n <= filled ? 'on' : undefined} />
      ))}
    </span>
  );
}

function Conditions({ data, group, nightNow }: { data: SolarData; group: string | null; nightNow: boolean | null }) {
  const t = useT();
  const find = (name: string, time: string) => data.bands.find((b) => b.name === name && b.time === time)?.condition ?? '';
  const word = (condition: string, level: Level) => (level === 'none' ? condition || '–' : t(`prop.cond.${level === 'good' ? 'Good' : level === 'fair' ? 'Fair' : 'Poor'}` as Key));
  return (
    <div className="prop-matrix" role="table" aria-label={t('prop.bands')}>
      <div className="prop-matrix-head" role="row">
        <span role="columnheader" />
        <span role="columnheader" className={nightNow === false ? 'prop-now' : undefined}>
          ☀ {t('prop.day')}
          {nightNow === false ? <small>{t('prop.here')}</small> : null}
        </span>
        <span role="columnheader" className={nightNow === true ? 'prop-now' : undefined}>
          ☾ {t('prop.night')}
          {nightNow === true ? <small>{t('prop.here')}</small> : null}
        </span>
      </div>
      {GROUPS.map((g) => (
        <div key={g} className={`prop-matrix-row${g === group ? ' prop-matrix-row--mine' : ''}`} role="row">
          <span role="rowheader" className="prop-group">
            {g}
            {g === group ? <small>{t('prop.tuned')}</small> : null}
          </span>
          {(['day', 'night'] as const).map((time) => {
            const condition = find(g, time);
            const level = conditionLevel(condition);
            const active = (time === 'night') === nightNow;
            return (
              <span key={time} role="cell" className={`prop-tile prop-tile--${level}${active ? ' prop-tile--active' : ''}`}>
                <Meter level={level} />
                <b>{word(condition, level)}</b>
              </span>
            );
          })}
        </div>
      ))}
    </div>
  );
}

function Stat({ value, label, hint }: { value: string; label: string; hint?: string }) {
  return (
    <div className="prop-stat" title={hint}>
      <strong className="data">{value}</strong>
      <span className="muted">{label}</span>
    </div>
  );
}

/** Propagazione e ricezione: cruscotto del sole, bande per giorno e notte, mappa di chi ti ha ricevuto. Tutto a richiesta: spento finche' non si accende. */
export function PropagationPage() {
  const t = useT();
  const client = useQueryClient();
  const prefs = usePrefs((s) => s.prefs);
  const status = useStation((s) => s.status);
  const online = useQuery({ queryKey: ['online'], queryFn: getOnline, retry: false });
  const solar = useQuery({ queryKey: ['online-solar'], queryFn: getSolar, retry: false, refetchInterval: 60_000, enabled: online.data?.solar === true });
  const psk = useQuery({ queryKey: ['online-psk'], queryFn: getPsk, retry: false, refetchInterval: 60_000, enabled: online.data?.psk === true });
  const refresh = useMutation({
    mutationFn: refreshOnline,
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ['online-solar'] });
      void client.invalidateQueries({ queryKey: ['online-psk'] });
    },
  });
  const options = useMemo(() => readMapOptions(), []);

  const pskState = psk.data?.psk ?? null;
  const rows = useMemo(() => (pskState ? pskRows(pskState.reports, status?.de_grid) : []), [pskState, status?.de_grid]);
  const receivers: MapReceiver[] = useMemo(() => latestPerReceiver(rows).map((r) => ({ call: r.receiver, grid: r.locator, snr: r.snr, band: r.band, km: r.km })), [rows]);
  // dalla piu' lontana alla piu' vicina (chi non ha un locatore noto va in fondo)
  const byDistance: PskRow[] = useMemo(() => [...rows].sort((a, b) => (b.km ?? -1) - (a.km ?? -1)), [rows]);
  const receivedColumns = useMemo(() => chunk(byDistance, 14), [byDistance]);

  if (online.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!online.data) return <p className="muted">…</p>;
  if (!online.data.psk && !online.data.solar) {
    return (
      <div className="prop-page">
        <h1 className="page-title">{t('prop.title')}</h1>
        <p className="muted">{t('prop.off')}</p>
        <a className="glass glass--link" href="#/impostazioni/online">
          {t('prop.settings')}
        </a>
      </div>
    );
  }

  const data = solar.data?.solar.data ?? null;
  const solarError = solar.data?.solar.error ?? null;
  const pskReady = pskState !== null && pskState.fetched_ms > 0;
  const pskHasRows = pskReady && rows.length > 0;
  const summary = summarize(rows);
  const counts = bandCounts(rows);
  const maxCount = Math.max(1, ...counts.map((c) => c.count));
  const best = bestSnr(rows);
  const farthestRow = summary.farthest ? rows.find((r) => r.receiver === summary.farthest?.call && r.km === summary.farthest.km) : undefined;
  const group = conditionGroup(status && status.dial_hz > 0 ? bandOf(status.dial_hz) : null);
  const home = status?.de_grid ? gridCenter(status.de_grid) : null;
  const nightNow = home ? isNight(home.lat, home.lon, new Date()) : null;

  return (
    <div className="prop-page">
      <header className="prop-head">
        <h1 className="page-title">{t('prop.title')}</h1>
        <button type="button" className="glass" onClick={() => refresh.mutate()} disabled={refresh.isPending}>
          {t('prop.refresh')}
        </button>
      </header>

      <div className="prop-columns">
        {online.data.solar || (online.data.psk && pskHasRows) ? (
          <aside className="prop-card prop-sidebar" aria-label={t('prop.solar')}>
            {online.data.solar ? (
              <>
                <h2>{t('prop.solar')}</h2>
                {data ? (
                  <>
                    <div className="prop-gauges">
                      <Gauge label={t('prop.sfi')} value={data.sfi} max={250} level={sfiLevel(data.sfi)} hint={t('prop.sfi.hint')} />
                      <Gauge label={t('prop.a')} value={data.a_index} max={50} level={aLevel(data.a_index)} hint={t('prop.a.hint')} />
                      <Gauge label={t('prop.k')} value={data.k_index} max={9} level={kLevel(data.k_index)} hint={t('prop.k.hint')} />
                    </div>
                    <div className="prop-facts">
                      <Stat label={t('prop.xray')} value={data.xray || '–'} />
                      <Stat label={t('prop.spots')} value={data.sunspots === null ? '–' : String(data.sunspots)} />
                      <Stat label={t('prop.wind')} value={data.solar_wind === null ? '–' : `${data.solar_wind.toFixed(0)} km/s`} />
                      <Stat label={t('prop.geomag')} value={data.geomag || '–'} />
                      <Stat label={t('prop.noise')} value={data.signal_noise || '–'} />
                      {data.aurora !== null ? <Stat label={t('prop.aurora')} value={String(data.aurora)} /> : null}
                    </div>
                    <h3>{t('prop.bands')}</h3>
                    <Conditions data={data} group={group} nightNow={nightNow} />
                    <p className="muted prop-credit">
                      {t('prop.updated', { when: data.updated || '–' })} · {t('prop.credit')}
                    </p>
                  </>
                ) : solarError ? (
                  <p className="notice">{t('prop.error', { message: solarError })}</p>
                ) : (
                  <p className="muted">{t('prop.waiting')}</p>
                )}
              </>
            ) : null}

            {online.data.psk && pskHasRows ? (
              <>
                <div className="prop-stats">
                  <Stat label={t('prop.stat.stations', { hours: pskState?.hours ?? 0 })} value={String(summary.stations)} />
                  <Stat
                    label={t('prop.stat.farthest')}
                    value={summary.farthest ? `${summary.farthest.km} km` : '–'}
                    hint={summary.farthest ? `${summary.farthest.call}${farthestRow?.country ? ` · ${farthestRow.country}` : ''}` : undefined}
                  />
                  <Stat label={t('prop.stat.farthestCall')} value={summary.farthest?.call ?? '–'} />
                  <Stat label={t('prop.stat.best')} value={best ? `${best.snr} dB` : '–'} hint={best?.call} />
                </div>
                <div className="prop-bars" aria-label={t('prop.stat.byBand')}>
                  {counts.map((c) => (
                    <div key={c.band} className="prop-bar">
                      <span className="data">{c.band}</span>
                      <span className="prop-bar-track">
                        <i style={{ width: `${(c.count / maxCount) * 100}%`, background: bandColor(c.band) }} />
                      </span>
                      <span className="data">{c.count}</span>
                    </div>
                  ))}
                </div>
                <div className="prop-legend" aria-hidden="true">
                  {BAND_LEGEND.filter(([b]) => counts.some((c) => c.band === b)).map(([b, color]) => (
                    <span key={b}>
                      <i style={{ background: color }} /> {b}
                    </span>
                  ))}
                </div>
              </>
            ) : null}
          </aside>
        ) : null}

        {online.data.psk ? (
          <section className="prop-card prop-main" aria-label={t('prop.psk')}>
            <h2>{t('prop.psk')}</h2>
            {pskReady ? (
              !pskHasRows ? (
                <p className="muted">{t('prop.psk.none', { hours: pskState?.hours ?? 0 })}</p>
              ) : (
                <>
                  <WorldMap squares={[]} stations={[]} receivers={receivers} homeGrid={status?.de_grid} onSelect={() => undefined} tiles={options.tiles} night={options.night} compact labels={mapLabels(t)} />
                  <div className="prop-received-columns">
                    {receivedColumns.map((col, i) => (
                      <div className="history-table-wrap prop-table-wrap" key={i}>
                        <table className="history-table prop-received-table">
                          <thead>
                            <tr>
                              <th>{t('prop.col.when')}</th>
                              <th>{t('prop.col.who')}</th>
                              <th>{t('prop.col.band')}</th>
                              <th className="num">{t('prop.col.snr')}</th>
                              <th className="num">{t('prop.col.distance')}</th>
                            </tr>
                          </thead>
                          <tbody>
                            {col.map((r) => {
                              const code = prefs.showFlag ? flagCode(r.country) : null;
                              return (
                                <tr key={`${r.receiver}-${r.time_s}-${r.freq_hz}`}>
                                  <td className="data">{formatClock(r.time_s * 1000)}</td>
                                  <td className="data">
                                    {code ? <span className={`fi fi-${code} prop-received-flag`} title={r.country ?? undefined} aria-hidden="true" /> : null}
                                    {r.receiver}
                                  </td>
                                  <td>
                                    <i className="prop-dot" style={{ background: bandColor(r.band) }} /> {r.band ?? '–'}
                                  </td>
                                  <td className="num data">{r.snr ?? '–'}</td>
                                  <td className="num data">{r.km !== null ? formatDistance(r.km, prefs.distanceUnit) : '–'}</td>
                                </tr>
                              );
                            })}
                          </tbody>
                        </table>
                      </div>
                    ))}
                  </div>
                </>
              )
            ) : pskState?.error ? (
              <p className="notice">{t('prop.error', { message: pskState.error })}</p>
            ) : (
              <p className="muted">{t('prop.waiting')}</p>
            )}
          </section>
        ) : null}
      </div>
    </div>
  );
}
