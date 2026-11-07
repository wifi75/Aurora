import { useGroups } from '../lib/detached';
import { DetachedNote } from '../components/DetachedNote';
import { DetachButton } from '../components/DetachButton';
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
import { useStatusPick } from '../state/station';
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

/** Il sole a colori per la colonna del giorno (al posto del simbolo ☀, che nei font cambia forma e colore). */
function SunIcon() {
  return (
    <svg className="prop-icon" viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
      <g stroke="#f5a623" strokeWidth="1.4" strokeLinecap="round">
        <path d="M8 1v2M8 13v2M1 8h2M13 8h2M3.05 3.05l1.4 1.4M11.55 11.55l1.4 1.4M3.05 12.95l1.4-1.4M11.55 4.45l1.4-1.4" />
      </g>
      <circle cx="8" cy="8" r="3.4" fill="#ffc83d" stroke="#f08c00" strokeWidth="0.8" />
    </svg>
  );
}

/** La luna piena a colori per la colonna della notte. */
function MoonIcon() {
  return (
    <svg className="prop-icon" viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
      <circle cx="8" cy="8" r="6.2" fill="#e9ecf5" stroke="#9aa6c4" strokeWidth="0.8" />
      <circle cx="6" cy="6.3" r="1.3" fill="#c9cfdf" />
      <circle cx="10.2" cy="9.6" r="1.6" fill="#c9cfdf" />
      <circle cx="6.8" cy="10.6" r="0.8" fill="#c9cfdf" />
    </svg>
  );
}

export function Conditions({ data, group, nightNow }: { data: SolarData; group: string | null; nightNow: boolean | null }) {
  const t = useT();
  const find = (name: string, time: string) => data.bands.find((b) => b.name === name && b.time === time)?.condition ?? '';
  const word = (condition: string, level: Level) => (level === 'none' ? condition || '–' : t(`prop.cond.${level === 'good' ? 'Good' : level === 'fair' ? 'Fair' : 'Poor'}` as Key));
  return (
    <div className="prop-matrix" role="table" aria-label={t('prop.bands')}>
      <div className="prop-matrix-head" role="row">
        <span role="columnheader" />
        <span role="columnheader" className={nightNow === false ? 'prop-now' : undefined}>
          <SunIcon />
          {t('prop.day')}
          {nightNow === false ? <small>{t('prop.here')}</small> : null}
        </span>
        <span role="columnheader" className={nightNow === true ? 'prop-now' : undefined}>
          <MoonIcon />
          {t('prop.night')}
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
export function PropagationPage({ solo = false }: { solo?: boolean } = {}) {
  const elsewhere = useGroups().some((g) => g.open && g.items.includes('propagation'));
  if (elsewhere && !solo) return <DetachedNote item="propagation" />;
  return <PropagationPageBody solo={solo} />;
}

function PropagationPageBody({ solo }: { solo: boolean }) {
  const t = useT();
  const client = useQueryClient();
  const prefs = usePrefs((s) => s.prefs);
  const status = useStatusPick(['de_grid', 'dial_hz']);
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
  // al massimo 4 colonne di uguale lunghezza: con piu' colonne strette la tabella era illeggibile
  const receivedColumns = useMemo(() => chunk(byDistance, Math.max(1, Math.ceil(byDistance.length / 4))), [byDistance]);

  if (online.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!online.data) return <p className="muted">…</p>;
  if (!online.data.psk && !online.data.solar) {
    return (
      <div className="prop-page">
        {solo ? null : (
          <h1 className="page-title page-title--tools">
            {t('prop.title')}
            <DetachButton item="propagation" />
          </h1>
        )}
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

  // in una finestra a parte solo la mappa di chi mi ha ricevuto, a tutta finestra
  if (solo) {
    return (
      <div className="map-page map-page--solo">
        <div className="worldmap-wrap">
          <WorldMap squares={[]} stations={[]} receivers={receivers} homeGrid={status?.de_grid} onSelect={() => undefined} tiles={options.tiles} mapStyleId={options.style} night={options.night} labels={mapLabels(t)} />
        </div>
      </div>
    );
  }

  return (
    <div className="prop-page">
      <header className="prop-head">
        {solo ? null : (
          <h1 className="page-title page-title--tools">
            {t('prop.title')}
            <DetachButton item="propagation" />
          </h1>
        )}
        <button type="button" className="glass" onClick={() => refresh.mutate()} title={t('tip.ui.refresh')} disabled={refresh.isPending}>
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
                  {/* la piu' lontana: distanza, e sotto chi mi ha ricevuto con la bandierina e l'orario */}
                  <div className="prop-stat" title={farthestRow?.country ?? undefined}>
                    <strong className="data">{summary.farthest ? formatDistance(summary.farthest.km, prefs.distanceUnit) : '–'}</strong>
                    <span className="muted">{t('prop.stat.farthest')}</span>
                    {summary.farthest ? (
                      <span className="data prop-stat-who">
                        {farthestRow && flagCode(farthestRow.country) ? <span className={`fi fi-${flagCode(farthestRow.country)} prop-received-flag`} aria-hidden="true" /> : null}
                        {summary.farthest.call}
                        {farthestRow?.country ? <span className="muted"> · {farthestRow.country}</span> : null}
                        {farthestRow ? <span className="muted"> · {formatClock(farthestRow.time_s * 1000)}</span> : null}
                      </span>
                    ) : null}
                  </div>
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
                  <WorldMap squares={[]} stations={[]} receivers={receivers} homeGrid={status?.de_grid} onSelect={() => undefined} tiles={options.tiles} mapStyleId={options.style} night={options.night} compact labels={mapLabels(t)} />
                  <div className="prop-received-columns">
                    {receivedColumns.map((col, i) => (
                      <div className="history-table-wrap prop-table-wrap" key={i}>
                        <table className="history-table prop-received-table">
                          <thead>
                            <tr>
                              <th>{t('prop.col.when')}</th>
                              <th title={t('tip.ui.colWho')}>{t('prop.col.who')}</th>
                              <th>{t('prop.col.band')}</th>
                              <th className="num" title={t('tip.ui.colSnr')}>{t('prop.col.snr')}</th>
                              <th className="num" title={t('tip.ui.colDistance')}>{t('prop.col.distance')}</th>
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
