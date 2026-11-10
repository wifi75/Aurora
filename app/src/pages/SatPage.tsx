import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo, useRef, useState } from 'react';
import { getSat, getSatPasses, getSatPositions, getSatTrack, putSat, satMapUrl, satRefreshTle, satUploadMap, type SatInfo, type SatSettings, type SatSnapshot } from '../api/client';
import { SatMap } from '../components/sat/SatMap';
import { SatRadar } from '../components/sat/SatRadar';
import { useT } from '../i18n';
import { compass, countdown, formatLat, formatLon, mhz, mmss } from '../lib/satGeo';
import { toast } from '../lib/toast';
import '../styles/sat.css';

const TRACK_MINUTES = 100;
const PREFERRED_MAP = 'Contour HiRes';

const clockOf = (unix: number) => {
  const d = new Date(unix * 1000);
  const p = (n: number) => String(n).padStart(2, '0');
  return { time: `${p(d.getUTCHours())}:${p(d.getUTCMinutes())}:${p(d.getUTCSeconds())}`, date: `${d.getUTCFullYear()}-${p(d.getUTCMonth() + 1)}-${p(d.getUTCDate())}` };
};

const localTime = (unix: number) => new Date(unix * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
const localDay = (unix: number) => new Date(unix * 1000).toLocaleDateString([], { day: '2-digit', month: '2-digit' });

/** La pagina Satelliti: mappa del mondo, passaggi, radar, dati del satellite, Doppler sulla radio e rotore che segue. */
export function SatPage() {
  const t = useT();
  const client = useQueryClient();
  const snap = useQuery({ queryKey: ['sat'], queryFn: getSat, retry: false, refetchInterval: 1000 });
  const data = snap.data;
  const selected = data?.settings.selected ?? null;
  const [hours, setHours] = useState(24);
  const [passMinEl, setPassMinEl] = useState(5);
  const positions = useQuery({ queryKey: ['sat-positions'], queryFn: getSatPositions, retry: false, refetchInterval: 2000, enabled: !!data });
  const track = useQuery({ queryKey: ['sat-track', selected], queryFn: () => getSatTrack(selected ?? 0, TRACK_MINUTES), retry: false, refetchInterval: 60_000, enabled: selected !== null && !!data });
  const passes = useQuery({ queryKey: ['sat-passes', selected, hours, passMinEl], queryFn: () => getSatPasses(selected ?? 0, hours, passMinEl), retry: false, refetchInterval: 60_000, enabled: selected !== null && !!data?.observer });

  return (
    <div className="sat-page">
      {snap.isError ? <p className="notice">{t('set.loadError')}</p> : null}
      {data ? <SatBody snap={data} positions={positions.data ?? []} track={track.data ?? []} passes={passes.data ?? []} hours={hours} setHours={setHours} passMinEl={passMinEl} setPassMinEl={setPassMinEl} onChanged={() => void client.invalidateQueries({ queryKey: ['sat'] })} /> : null}
    </div>
  );
}

interface BodyProps {
  snap: SatSnapshot;
  positions: Awaited<ReturnType<typeof getSatPositions>>;
  track: [number, number][];
  passes: Awaited<ReturnType<typeof getSatPasses>>;
  hours: number;
  setHours(h: number): void;
  passMinEl: number;
  setPassMinEl(e: number): void;
  onChanged(): void;
}

function SatBody({ snap, positions, track, passes, hours, setHours, passMinEl, setPassMinEl, onChanged }: BodyProps) {
  const t = useT();
  const client = useQueryClient();
  const s = snap.settings;
  const now = snap.now;
  const look = now.look;
  const [search, setSearch] = useState('');
  const [busy, setBusy] = useState(false);
  const [downDraft, setDownDraft] = useState<number | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const info: SatInfo | undefined = snap.catalog.find((c) => c.norad === s.selected);
  const tp = info?.transponders[s.transponder] ?? info?.transponders[0];
  const clock = clockOf(snap.sky.time_unix);

  const save = (patch: Partial<SatSettings>) => {
    void putSat({ ...s, ...patch }).then((next) => {
      client.setQueryData(['sat'], next);
      onChanged();
    });
  };

  // la mappa: quella scelta, altrimenti «Contour HiRes», altrimenti la prima che c'e'
  const mapName = useMemo(() => {
    if (s.map && snap.maps.includes(s.map)) return s.map;
    return snap.maps.find((m) => m.startsWith(PREFERRED_MAP)) ?? snap.maps[0] ?? null;
  }, [s.map, snap.maps]);

  const downValue = downDraft ?? s.down_hz ?? (tp ? (tp.down_low_hz + tp.down_high_hz) / 2 : 0);

  const list = useMemo(() => {
    const f = search.trim().toUpperCase();
    return snap.listing.filter((x) => f === '' || x.name.toUpperCase().includes(f) || String(x.norad).includes(f));
  }, [snap.listing, search]);

  const refreshTle = () => {
    setBusy(true);
    satRefreshTle().then(
      (n) => {
        toast(t('sat.tle.done', { n }), 'ok');
        onChanged();
      },
      (e: unknown) => toast(t('sat.tle.error', { why: e instanceof Error ? e.message : String(e) }), 'error'),
    ).finally(() => setBusy(false));
  };

  const addMap = (file: File | undefined) => {
    if (!file) return;
    void satUploadMap(file.name, file).then(
      () => {
        save({ map: file.name });
        onChanged();
      },
      () => toast(t('sat.map.addHint'), 'error'),
    );
  };

  const ageText = snap.tle.age_s === null ? null : countdown(snap.tle.age_s);
  const next = now.next_pass;
  const inPass = now.visible && next !== null && next.aos_unix <= now.time_unix;

  return (
    <>
      <header className="sat-head">
        <h1>{t('sat.title')}</h1>
        <div className="sat-clock" aria-label="UTC">
          <b>{clock.time}</b>
          <span>
            {clock.date} UTC
          </span>
        </div>
        <div className="sat-tle">
          <span className="sat-muted">{ageText ? t('sat.tle.age', { t: ageText }) : t('sat.tle.none')}</span>
          <button type="button" className="glass sstv-btn" disabled={busy} onClick={refreshTle}>
            {t('sat.tle.refresh')}
          </button>
        </div>
      </header>

      {!snap.observer ? <p className="notice">{t('sat.noObserver')}</p> : null}

      <div className="sat-grid">
        <section className="sat-main">
          <div className="sat-panel sat-mapwrap">
            {mapName ? null : <p className="sat-muted sat-nomap">{t('sat.map.none')}</p>}
            <SatMap
              mapUrl={mapName ? satMapUrl(mapName) : null}
              sun={{ lat: snap.sky.sun_lat_deg, lon: snap.sky.sun_lon_deg }}
              moon={{ lat: snap.sky.moon_lat_deg, lon: snap.sky.moon_lon_deg }}
              observer={snap.observer ? { lat: snap.observer.lat_deg, lon: snap.observer.lon_deg } : null}
              stationLabel={t('sat.station')}
              positions={positions}
              selected={s.selected}
              track={track}
              trackNowIndex={TRACK_MINUTES}
              onSelect={(norad) => save({ selected: norad, transponder: 0, down_hz: null })}
            />
            <div className="sat-maptools">
              <label>
                <span>{t('sat.map.pick')}</span>
                <select value={mapName ?? ''} onChange={(e) => save({ map: e.target.value })} disabled={snap.maps.length === 0}>
                  {snap.maps.map((m) => (
                    <option key={m} value={m}>
                      {m.replace(/\.[a-z]+$/i, '')}
                    </option>
                  ))}
                </select>
              </label>
              <button type="button" className="glass sstv-btn" title={t('sat.map.addHint')} onClick={() => fileInput.current?.click()}>
                {t('sat.map.add')}
              </button>
              <input ref={fileInput} type="file" accept=".jpg,.jpeg,.png,.bmp,.webp" hidden onChange={(e) => addMap(e.target.files?.[0])} />
              <span className="sat-legend">
                <i className="sat-dot sat-dot--sun" /> {t('sat.sun')} <i className="sat-dot sat-dot--moon" /> {t('sat.moon')}
              </span>
            </div>
          </div>

          <div className="sat-panel">
            <div className="sat-passhead">
              <h2>{t('sat.pass.title')}{info ? ` · ${info.name}` : ''}</h2>
              <label>
                <span>{t('sat.pass.hours')}</span>
                <select value={hours} onChange={(e) => setHours(Number(e.target.value))}>
                  {[12, 24, 48, 72].map((h) => (
                    <option key={h} value={h}>
                      {h}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span>{t('sat.pass.minEl')}</span>
                <input type="number" min={0} max={80} value={passMinEl} onChange={(e) => setPassMinEl(Math.max(0, Math.min(80, Number(e.target.value) || 0)))} />
              </label>
            </div>
            {next ? (
              <p className={`sat-next${inPass ? ' sat-next--now' : ''}`}>
                <b>{inPass ? t('sat.pass.now') : t('sat.pass.next')}</b>{' '}
                {inPass ? t('sat.pass.endsIn', { t: countdown(next.los_unix - now.time_unix) }) : t('sat.pass.in', { t: countdown(next.aos_unix - now.time_unix) })} · {t('sat.d.az')} {Math.round(next.aos_az_deg)}° → {Math.round(next.los_az_deg)}° · {t('sat.pass.max')} {Math.round(next.max_el_deg)}°
              </p>
            ) : null}
            <div className="sat-table">
              <table>
                <thead>
                  <tr>
                    <th>{t('sat.pass.aos')}</th>
                    <th>{t('sat.pass.max')}</th>
                    <th>{t('sat.pass.los')}</th>
                    <th>{t('sat.pass.dur')}</th>
                    <th>{t('sat.pass.az')}</th>
                  </tr>
                </thead>
                <tbody>
                  {passes.map((p) => (
                    <tr key={p.aos_unix} className={p.aos_unix <= now.time_unix && p.los_unix > now.time_unix ? 'on' : ''}>
                      <td>
                        {localDay(p.aos_unix)} <b>{localTime(p.aos_unix)}</b>
                      </td>
                      <td>
                        <b>{Math.round(p.max_el_deg)}°</b>
                      </td>
                      <td>{localTime(p.los_unix)}</td>
                      <td>{mmss(p.duration_s)}</td>
                      <td>
                        {Math.round(p.aos_az_deg)}° {compass(p.aos_az_deg)} → {Math.round(p.los_az_deg)}° {compass(p.los_az_deg)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {passes.length === 0 ? <p className="sat-muted">{t('sat.pass.empty')}</p> : null}
            </div>
          </div>

          {info ? (
            <div className="sat-panel">
              <h2>
                {t('sat.info.title')} · {info.name}
              </h2>
              <p className="sat-muted">{info.notes}</p>
              <div className="sat-table">
                <table>
                  <thead>
                    <tr>
                      <th>{t('sat.ctl.transponder')}</th>
                      <th>{t('sat.info.up')} (MHz)</th>
                      <th>{t('sat.info.down')} (MHz)</th>
                    </tr>
                  </thead>
                  <tbody>
                    {info.transponders.map((x, i) => (
                      <tr key={x.name + i} className={i === s.transponder ? 'on' : ''}>
                        <td>
                          {x.name} <span className="sat-muted">{x.mode}{x.inverting ? ` · ${t('sat.info.inverting')}` : ''}{x.tone_hz ? ` · ${x.tone_hz} Hz` : ''}</span>
                        </td>
                        <td>{x.up_low_hz === x.up_high_hz ? (x.up_low_hz / 1e6).toFixed(3) : `${(x.up_low_hz / 1e6).toFixed(3)} – ${(x.up_high_hz / 1e6).toFixed(3)}`}</td>
                        <td>{x.down_low_hz === x.down_high_hz ? (x.down_low_hz / 1e6).toFixed(3) : `${(x.down_low_hz / 1e6).toFixed(3)} – ${(x.down_high_hz / 1e6).toFixed(3)}`}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <p className="sat-muted sat-small">{t('sat.info.verify')}</p>
            </div>
          ) : null}
        </section>

        <aside className="sat-side">
          <div className="sat-panel">
            <h2>{t('sat.list.title')}</h2>
            <input className="sat-search" placeholder={t('sat.list.search')} aria-label={t('sat.list.search')} value={search} onChange={(e) => setSearch(e.target.value)} />
            <ul className="sat-list" title={t('sat.list.pick')}>
              {list.map((x) => (
                <li key={x.norad} className={x.norad === s.selected ? 'on' : ''}>
                  <input
                    type="checkbox"
                    checked={s.tracked.includes(x.norad)}
                    aria-label={x.name}
                    onChange={(e) => save({ tracked: e.target.checked ? [...s.tracked, x.norad] : s.tracked.filter((n) => n !== x.norad) })}
                  />
                  <button type="button" onClick={() => save({ selected: x.norad, transponder: 0, down_hz: null })}>
                    {x.name}
                    {x.has_tle ? null : <small> · {t('sat.list.noTle')}</small>}
                  </button>
                </li>
              ))}
            </ul>
          </div>

          <div className="sat-panel">
            <h2>{now.name || t('sat.data.title')}</h2>
            {look ? (
              <dl className="sat-data">
                <div><dt>{t('sat.d.lon')}</dt><dd>{formatLon(look.lon_deg)}</dd></div>
                <div><dt>{t('sat.d.lat')}</dt><dd>{formatLat(look.lat_deg)}</dd></div>
                <div><dt>{t('sat.d.alt')}</dt><dd>{look.alt_km.toFixed(1)} km</dd></div>
                <div><dt>{t('sat.d.az')}</dt><dd>{look.az_deg.toFixed(1)}° {compass(look.az_deg)}</dd></div>
                <div><dt>{t('sat.d.el')}</dt><dd className={look.el_deg >= 0 ? 'sat-up' : ''}>{look.el_deg.toFixed(1)}°</dd></div>
                <div><dt>{t('sat.d.range')}</dt><dd>{look.range_km.toFixed(0)} km</dd></div>
                <div><dt>{t('sat.d.rate')}</dt><dd>{look.range_rate_kms.toFixed(3)} km/s</dd></div>
                <div><dt>{t('sat.d.speed')}</dt><dd>{look.speed_kms.toFixed(3)} km/s</dd></div>
                <div><dt>{t('sat.d.direction')}</dt><dd>{look.rising ? t('sat.d.approaching') : t('sat.d.receding')}</dd></div>
                <div><dt>&nbsp;</dt><dd className={now.visible ? 'sat-up' : 'sat-muted'}>{now.visible ? t('sat.d.visible') : t('sat.d.hidden')}</dd></div>
              </dl>
            ) : (
              <p className="sat-muted">{snap.observer ? t('sat.list.noTle') : t('sat.noObserver')}</p>
            )}
            {snap.details ? (
              <>
                <h3>{t('sat.orbit.title')}</h3>
                <dl className="sat-data sat-data--small">
                  <div><dt>{t('sat.orbit.incl')}</dt><dd>{snap.details.inclination_deg.toFixed(3)}°</dd></div>
                  <div><dt>{t('sat.orbit.ecc')}</dt><dd>{snap.details.eccentricity.toFixed(5)}</dd></div>
                  <div><dt>{t('sat.orbit.period')}</dt><dd>{snap.details.period_min.toFixed(2)} min</dd></div>
                  <div><dt>{t('sat.orbit.revs')}</dt><dd>{snap.details.revs_per_day.toFixed(5)}</dd></div>
                  <div><dt>{t('sat.orbit.epoch')}</dt><dd>{new Date(snap.details.epoch_unix * 1000).toISOString().slice(0, 16).replace('T', ' ')}</dd></div>
                </dl>
              </>
            ) : null}
          </div>

          <div className="sat-panel sat-radarpanel">
            <SatRadar path={now.next_path} now={look ? { az: look.az_deg, el: look.el_deg } : null} rotor={s.rotor_on && now.rotor.ready ? { az: now.rotor.az_deg, el: now.rotor.el_deg } : null} />
          </div>

          <div className="sat-panel">
            <h2>{t('sat.ctl.title')}</h2>
            {info && info.transponders.length > 0 ? (
              <>
                <label className="sat-field">
                  <span>{t('sat.ctl.transponder')}</span>
                  <select value={s.transponder} onChange={(e) => save({ transponder: Number(e.target.value), down_hz: null })}>
                    {info.transponders.map((x, i) => (
                      <option key={x.name + i} value={i}>
                        {x.name} — {x.mode}
                      </option>
                    ))}
                  </select>
                </label>
                {tp && tp.down_high_hz - tp.down_low_hz > 1 ? (
                  <label className="sat-field">
                    <span>
                      {t('sat.ctl.downlink')}: <b>{mhz(downValue)}</b> MHz
                    </span>
                    <input
                      type="range"
                      min={tp.down_low_hz}
                      max={tp.down_high_hz}
                      step={100}
                      value={downValue}
                      onChange={(e) => setDownDraft(Number(e.target.value))}
                      onPointerUp={() => {
                        if (downDraft !== null) save({ down_hz: downDraft });
                        setDownDraft(null);
                      }}
                      onKeyUp={() => {
                        if (downDraft !== null) save({ down_hz: downDraft });
                        setDownDraft(null);
                      }}
                    />
                  </label>
                ) : null}
                <label className="sat-field">
                  <span>{t('sat.ctl.txOffset')}</span>
                  <input type="number" step={10} value={s.tx_offset_hz} onChange={(e) => save({ tx_offset_hz: Number(e.target.value) || 0 })} />
                </label>
              </>
            ) : (
              <p className="sat-muted">{t('sat.ctl.noTransponder')}</p>
            )}
            <label className="sat-field">
              <span>{t('sat.ctl.minEl')}</span>
              <input type="number" min={0} max={60} value={s.min_el_deg} onChange={(e) => save({ min_el_deg: Math.max(0, Math.min(60, Number(e.target.value) || 0)) })} />
            </label>

            <label className="sat-switch">
              <input type="checkbox" checked={s.doppler_on} onChange={(e) => save({ doppler_on: e.target.checked })} />
              <span>{t('sat.ctl.doppler')}</span>
            </label>
            <p className={`sat-status ${now.radio.ready ? 'ok' : 'warn'}`}>{now.radio.ready ? t('sat.ctl.radioReady') : t('sat.ctl.radioNot')}</p>
            {s.doppler_on ? (
              now.plan && now.visible ? (
                <dl className="sat-data sat-data--small sat-freqs">
                  <div>
                    <dt>{t('sat.ctl.rx')}</dt>
                    <dd>
                      <b>{mhz(now.plan.rx_hz)}</b> MHz <small>({now.plan.rx_shift_hz >= 0 ? '+' : ''}{now.plan.rx_shift_hz.toFixed(0)} Hz)</small>
                    </dd>
                  </div>
                  {Math.abs(now.plan.up_nominal_hz - now.plan.down_nominal_hz) > 1000 ? (
                    <div>
                      <dt>{t('sat.ctl.tx')}</dt>
                      <dd>
                        <b>{mhz(now.plan.tx_hz)}</b> MHz <small>({now.plan.tx_shift_hz >= 0 ? '+' : ''}{now.plan.tx_shift_hz.toFixed(0)} Hz)</small>
                      </dd>
                    </div>
                  ) : null}
                  <div>
                    <dt>{t('sat.ctl.nominal')}</dt>
                    <dd>
                      {mhz(now.plan.down_nominal_hz)} / {mhz(now.plan.up_nominal_hz)}
                    </dd>
                  </div>
                  <div>
                    <dt>&nbsp;</dt>
                    <dd className={now.radio_active ? 'sat-up' : 'sat-muted'}>{now.radio_active ? t('sat.ctl.active') : (now.radio.error ?? '')}</dd>
                  </div>
                </dl>
              ) : (
                <p className="sat-muted">{t('sat.ctl.waiting', { el: s.min_el_deg })}</p>
              )
            ) : null}

            <label className="sat-switch">
              <input type="checkbox" checked={s.rotor_on} onChange={(e) => save({ rotor_on: e.target.checked })} />
              <span>{t('sat.ctl.rotor')}</span>
            </label>
            <label className="sat-field sat-field--check">
              <input type="checkbox" checked={s.rotor_preposition} onChange={(e) => save({ rotor_preposition: e.target.checked })} />
              <span>{t('sat.ctl.prepoint')}</span>
            </label>
            <p className={`sat-status ${now.rotor.ready ? 'ok' : 'warn'}`}>{now.rotor.ready ? t('sat.ctl.rotorReady') : t('sat.ctl.rotorNot')}</p>
            {s.rotor_on && now.rotor.ready ? (
              <p className="sat-muted">{t('sat.ctl.rotorTo', { az: Math.round(now.rotor.az_deg), el: Math.round(now.rotor.el_deg) })}</p>
            ) : null}
            {now.rotor.error ? <p className="sat-status warn">{now.rotor.error}</p> : null}
          </div>
        </aside>
      </div>
    </>
  );
}
