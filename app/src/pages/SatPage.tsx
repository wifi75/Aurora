import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Icon } from '../components/Icon';
import { ColumnsMenu, MovableColGroup, MovableTh, useMovableColumns } from '../components/MovableColumns';
import { useEffect, useMemo, useRef, useState } from 'react';
import { getSat, getSatPasses, getSatPassesAll, getSatPositions, getSatTrack, putSat, satMapUrl, satRefreshTle, type SatInfo, type SatSettings, type SatSnapshot } from '../api/client';
import { SatMap } from '../components/sat/SatMap';
import { SatRadar } from '../components/sat/SatRadar';
import { SatAlertsPanel } from '../components/sat/SatAlertsPanel';
import { DashGrid, type DashWidget } from '../components/sat/DashGrid';
import { SAT_MAX_COLS, SAT_MAX_ROWS, SAT_MINS, defaultLayout, loadLayout, saveLayout, type StoredLayout } from '../lib/satLayout';
import { mergePasses, type AlertPass } from '../lib/satAlerts';
import { useSatAlerts } from '../lib/useSatAlerts';
import { clearSatellites, groupListing, normalizeSatSelection, pickSatellite, setTracked } from '../lib/satSelection';
import { useStatusPick } from '../state/station';
import { openSettingsWindow } from '../lib/settingsWindow';
import { useT } from '../i18n';
import { compass, countdown, formatLat, formatLon, mhz, mmss } from '../lib/satGeo';
import { loadBorders, PREFERRED_MAP, resolveMap } from '../lib/satMapChoice';
import { toast } from '../lib/toast';
import '../styles/sat.css';

const TRACK_MINUTES = 100;

const clockOf = (unix: number) => {
  const d = new Date(unix * 1000);
  const p = (n: number) => String(n).padStart(2, '0');
  return { time: `${p(d.getUTCHours())}:${p(d.getUTCMinutes())}:${p(d.getUTCSeconds())}`, date: `${d.getUTCFullYear()}-${p(d.getUTCMonth() + 1)}-${p(d.getUTCDate())}` };
};

const localTime = (unix: number) => new Date(unix * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
/** Sotto questa larghezza della finestra le schede si impilano in una colonna. */
const NARROW_QUERY = '(max-width: 900px)';

function useNarrow(): boolean {
  const [narrow, setNarrow] = useState(() => typeof window.matchMedia === 'function' && window.matchMedia(NARROW_QUERY).matches);
  useEffect(() => {
    if (typeof window.matchMedia !== 'function') return;
    const mq = window.matchMedia(NARROW_QUERY);
    const on = () => setNarrow(mq.matches);
    mq.addEventListener('change', on);
    on();
    return () => mq.removeEventListener('change', on);
  }, []);
  return narrow;
}

const localDay = (unix: number) => new Date(unix * 1000).toLocaleDateString([], { day: '2-digit', month: '2-digit' });

type PassCol = 'sat' | 'date' | 'aos' | 'los' | 'dur' | 'max' | 'az';
const PASS_COLUMNS = ['date', 'aos', 'los', 'dur', 'max', 'az'] as const;
const ALL_COLUMNS = ['sat', ...PASS_COLUMNS] as const;
/** Larghezze di partenza delle colonne, in em del corpo della tabella (13 px): Data 54 px, Inizio e Fine 66, Durata 52, Max 50, Azimut 110, Satellite 120. */
const PASS_EM: Record<PassCol, number> = { sat: 9.3, date: 4.2, aos: 5.1, los: 5.1, dur: 4.4, max: 4.4, az: 8.5 };
const COL_LABEL: Record<PassCol, 'sat.all.sat' | 'sat.pass.date' | 'sat.pass.aos' | 'sat.pass.los' | 'sat.pass.dur' | 'sat.pass.max' | 'sat.pass.az'> = {
  sat: 'sat.all.sat',
  date: 'sat.pass.date',
  aos: 'sat.pass.aos',
  los: 'sat.pass.los',
  dur: 'sat.pass.dur',
  max: 'sat.pass.max',
  az: 'sat.pass.az',
};
/** Doppio clic su un titolo (non sul bordo): ripristina le colonne. */
const resetOnTitle = (e: { target: EventTarget }, reset: () => void) => {
  if (!(e.target as HTMLElement).closest('.cl-grip')) reset();
};

interface PassRow {
  aos_unix: number;
  los_unix: number;
  duration_s: number;
  max_el_deg: number;
  aos_az_deg: number;
  los_az_deg: number;
}

/** Una cella delle tabelle dei passaggi: stesso testo nelle due tabelle, con il suggerimento per quello che l'ellissi puo' accorciare. */
function passCell(c: PassCol, p: PassRow, name: string) {
  switch (c) {
    case 'sat':
      return (
        <td key={c} title={name}>
          <b>{name}</b>
        </td>
      );
    case 'date':
      return <td key={c}>{localDay(p.aos_unix)}</td>;
    case 'aos':
      return (
        <td key={c}>
          <b>{localTime(p.aos_unix)}</b>
        </td>
      );
    case 'los':
      return <td key={c}>{localTime(p.los_unix)}</td>;
    case 'dur':
      return <td key={c}>{mmss(p.duration_s)}</td>;
    case 'max':
      return (
        <td key={c}>
          <b>{Math.round(p.max_el_deg)}°</b>
        </td>
      );
    case 'az': {
      const tip = `${Math.round(p.aos_az_deg)}° ${compass(p.aos_az_deg)} → ${Math.round(p.los_az_deg)}° ${compass(p.los_az_deg)}`;
      return (
        <td key={c} title={tip}>
          {Math.round(p.aos_az_deg)}° → {Math.round(p.los_az_deg)}°
        </td>
      );
    }
  }
}

/** La pagina Satelliti: mappa del mondo, passaggi, radar, dati del satellite, Doppler sulla radio e rotore che segue. */
export function SatPage() {
  const t = useT();
  const client = useQueryClient();
  const snap = useQuery({ queryKey: ['sat'], queryFn: getSat, retry: false, refetchInterval: 1000 });
  const data = snap.data;
  const selected = data ? normalizeSatSelection(data.settings).selected : null;
  const [hours, setHours] = useState(24);
  const [passMinEl, setPassMinEl] = useState(5);
  const [allHours, setAllHours] = useState(24);
  const positions = useQuery({ queryKey: ['sat-positions'], queryFn: getSatPositions, retry: false, refetchInterval: 2000, enabled: !!data });
  const track = useQuery({ queryKey: ['sat-track', selected], queryFn: () => getSatTrack(selected ?? 0, TRACK_MINUTES), retry: false, refetchInterval: 60_000, enabled: selected !== null && !!data });
  const passes = useQuery({ queryKey: ['sat-passes', selected, hours, passMinEl], queryFn: () => getSatPasses(selected ?? 0, hours, passMinEl), retry: false, refetchInterval: 60_000, enabled: selected !== null && !!data?.observer });

  const passesAll = useQuery({ queryKey: ['sat-passes-all', allHours, passMinEl, data?.settings.tracked.join(',')], queryFn: () => getSatPassesAll(allHours, passMinEl), retry: false, refetchInterval: 60_000, enabled: !!data?.observer && data.settings.tracked.length > 0 });

  return (
    <div className="sat-page">
      {snap.isError ? <p className="notice">{t('set.loadError')}</p> : null}
      {data ? <SatBody snap={data} positions={positions.data ?? []} track={track.data ?? []} passes={passes.data ?? []} passesAll={passesAll.data ?? []} allHours={allHours} setAllHours={setAllHours} hours={hours} setHours={setHours} passMinEl={passMinEl} setPassMinEl={setPassMinEl} onChanged={() => void client.invalidateQueries({ queryKey: ['sat'] })} /> : null}
    </div>
  );
}

interface BodyProps {
  snap: SatSnapshot;
  positions: Awaited<ReturnType<typeof getSatPositions>>;
  track: [number, number][];
  passes: Awaited<ReturnType<typeof getSatPasses>>;
  passesAll: Awaited<ReturnType<typeof getSatPassesAll>>;
  allHours: number;
  setAllHours(h: number): void;
  hours: number;
  setHours(h: number): void;
  passMinEl: number;
  setPassMinEl(e: number): void;
  onChanged(): void;
}

function SatBody({ snap, positions, track, passes, passesAll, allHours, setAllHours, hours, setHours, passMinEl, setPassMinEl, onChanged }: BodyProps) {
  const t = useT();
  const client = useQueryClient();
  const s = snap.settings;
  const now = snap.now;
  const look = now.look;
  const [search, setSearch] = useState('');
  const [busy, setBusy] = useState(false);
  const [downDraft, setDownDraft] = useState<number | null>(null);
  // la casella e' la verita': un satellite scelto ma non spuntato vale come nessuno
  const sel = normalizeSatSelection(s).selected;
  const info: SatInfo | undefined = sel === null ? undefined : snap.catalog.find((c) => c.norad === sel);
  const shownPositions = useMemo(() => positions.filter((p) => s.tracked.includes(p.norad)), [positions, s.tracked]);
  const tp = info?.transponders[s.transponder] ?? info?.transponders[0];
  const clock = clockOf(snap.sky.time_unix);
  const myCall = (useStatusPick(['de_call'])?.de_call ?? '').trim().toUpperCase();

  // avvisi: i passaggi di tutti i satelliti spuntati e, se serve, quelli del satellite scelto (anche se non e' spuntato)
  const alertPasses = useMemo<AlertPass[]>(() => {
    const all = passesAll.map((p) => ({ norad: p.norad, name: p.name, aos_unix: p.aos_unix, los_unix: p.los_unix, max_el_deg: p.max_el_deg }));
    const own = sel === null ? [] : passes.map((p) => ({ norad: sel, name: info?.name ?? now.name, aos_unix: p.aos_unix, los_unix: p.los_unix, max_el_deg: p.max_el_deg }));
    return mergePasses(all, own);
  }, [passesAll, passes, sel, info?.name, now.name]);
  const alerts = useSatAlerts({ passes: alertPasses, positions: shownPositions, tracked: s.tracked, selected: sel, minEl: s.min_el_deg, serverNow: now.time_unix });

  const save = (patch: Partial<SatSettings>) => {
    void putSat({ ...s, ...patch }).then((next) => {
      client.setQueryData(['sat'], next);
      onChanged();
    });
  };

  const pick = (norad: number) => save({ ...pickSatellite(s, norad), transponder: 0, down_hz: null });

  // all'apertura: se il satellite scelto non e' spuntato, core e interfaccia devono concordare su "nessuno"
  const fixed = useRef(false);
  useEffect(() => {
    if (fixed.current || !normalizeSatSelection(s).changed) return;
    fixed.current = true;
    save({ selected: normalizeSatSelection(s).selected, transponder: 0, down_hz: null });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [s.selected, s.tracked]);

  const groups = useMemo(() => {
    const f = search.trim().toUpperCase();
    const ok = (x: { name: string; norad: number }) => f === '' || x.name.toUpperCase().includes(f) || String(x.norad).includes(f);
    const g = groupListing(snap.listing, s.tracked);
    return { ticked: g.ticked.filter(ok), rest: g.rest.filter(ok), anyTicked: g.ticked.length > 0 };
  }, [snap.listing, s.tracked, search]);

  // la mappa: quella scelta (Terra, semplice o un'immagine dell'utente); senza scelta «Contour HiRes» se c'e', altrimenti la Terra
  const mapChoice = useMemo(() => resolveMap(s.map, snap.maps, PREFERRED_MAP), [s.map, snap.maps]);
  const [borders] = useState(loadBorders);

  const downValue = downDraft ?? s.down_hz ?? (tp ? (tp.down_low_hz + tp.down_high_hz) / 2 : 0);

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

  const ageText = snap.tle.age_s === null ? null : countdown(snap.tle.age_s);
  const next = now.next_pass;
  const inPass = now.visible && next !== null && next.aos_unix <= now.time_unix;

  const [stored, setStored] = useState<StoredLayout>(loadLayout);
  const narrow = useNarrow();
  const setLayout = (next: StoredLayout) => {
    setStored(next);
    saveLayout(next);
  };

  const row = (x: { norad: number; name: string; has_tle: boolean }) => (
    <li key={x.norad} className={x.norad === sel ? 'on' : ''}>
      <input type="checkbox" checked={s.tracked.includes(x.norad)} aria-label={x.name} onChange={(e) => {
          const r = setTracked(s, x.norad, e.target.checked);
          save({ ...r, ...(r.selected !== sel ? { transponder: 0, down_hz: null } : {}) });
        }} />
      <button type="button" data-tip={t('sat.list.pick')} onClick={() => pick(x.norad)}>
        {x.name}
        {x.has_tle ? null : <small> · {t('sat.list.noTle')}</small>}
      </button>
    </li>
  );

  const passCols = useMovableColumns({ storageKey: 'aurora.sat.grid.passes', columns: PASS_COLUMNS, defaultEm: PASS_EM, elastic: 'az' });
  const allCols = useMovableColumns({ storageKey: 'aurora.sat.grid.all', columns: ALL_COLUMNS, defaultEm: PASS_EM, elastic: 'az' });
  const defaultWidths = useMemo(() => Object.fromEntries(defaultLayout().map((i) => [i.id, i.w])), []);

  const widgets: DashWidget[] = [
    {
      id: 'map',
      title: t('sat.map.pick'),
      bare: true,
      ratio: 2,
      ...SAT_MINS.map,
      maxRows: SAT_MAX_ROWS.map,
      maxW: SAT_MAX_COLS.map,
      children: (
          <SatMap
            mapUrl={mapChoice.kind === 'file' ? satMapUrl(mapChoice.name) : null}
            terra={mapChoice.kind === 'terra'}
            borders={borders}
            sun={{ lat: snap.sky.sun_lat_deg, lon: snap.sky.sun_lon_deg }}
            moon={{ lat: snap.sky.moon_lat_deg, lon: snap.sky.moon_lon_deg }}
            observer={snap.observer ? { lat: snap.observer.lat_deg, lon: snap.observer.lon_deg } : null}
            stationLabel={myCall !== '' ? myCall : t('sat.station')}
            positions={shownPositions}
            selected={sel}
            track={track}
            trackNowIndex={TRACK_MINUTES}
            alerts={alerts.views}
            zoomMode={alerts.settings.zoom}
            onSelect={(norad) => pick(norad)}
          />
      ),
    },
    {
      id: 'list',
      title: t('sat.list.title'),
      ...SAT_MINS.list,
      maxRows: SAT_MAX_ROWS.list,
      maxW: SAT_MAX_COLS.list,
      children: (
        <>
  <input className="sat-search" placeholder={t('sat.list.search')} aria-label={t('sat.list.search')} value={search} onChange={(e) => setSearch(e.target.value)} />
  <button type="button" className="sat-clear" disabled={s.tracked.length === 0 && sel === null} onClick={() => save({ ...clearSatellites(), transponder: 0, down_hz: null })}>
    {t('sat.list.clear')}
  </button>
  <ul className="sat-list">
    {groups.anyTicked && groups.ticked.length > 0 ? <li className="sat-list-head">{t('sat.list.ticked')}</li> : null}
    {groups.ticked.map((x) => row(x))}
    {groups.anyTicked && groups.rest.length > 0 ? <li className="sat-list-head">{t('sat.list.all')}</li> : null}
    {groups.rest.map((x) => row(x))}
  </ul>
        </>
      ),
    },
    {
      id: 'data',
      title: sel === null ? t('sat.data.title') : now.name || t('sat.data.title'),
      ...SAT_MINS.data,
      maxRows: SAT_MAX_ROWS.data,
      maxW: SAT_MAX_COLS.data,
      children: (
        <>
  {sel === null ? (
    <p className="sat-muted">{t('sat.none')}</p>
  ) : look ? (
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
  {sel !== null && snap.details ? (
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
        </>
      ),
    },
    {
      id: 'pass',
      title: `${t('sat.pass.title')}${info ? ` · ${info.name}` : ''}`,
      actions: (
        <>
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
        </>
      ),
      ...SAT_MINS.pass,
      maxRows: SAT_MAX_ROWS.pass,
      maxW: SAT_MAX_COLS.pass,
      children: sel === null ? (
        <p className="sat-muted">{t('sat.none')}</p>
      ) : (
        <>
    {next ? (
      <p className={`sat-next sat-next--compact${inPass ? ' sat-next--now' : ''}`}>
        <span>{inPass ? t('sat.pass.now') : t('sat.pass.next')}</span>{' '}
        <b className="sat-count">{inPass ? t('sat.pass.endsIn', { t: countdown(next.los_unix - now.time_unix) }) : t('sat.pass.in', { t: countdown(next.aos_unix - now.time_unix) })}</b>
        <span className="sat-next-rest"> · {Math.round(next.aos_az_deg)}° → {Math.round(next.los_az_deg)}° · {t('sat.pass.max')} {Math.round(next.max_el_deg)}°</span>
      </p>
    ) : null}
    <div className="sat-table sat-table--pass">
      <table ref={passCols.tableRef} className="mv-grid sat-grid-table" style={passCols.tableStyle}>
        <MovableColGroup cols={passCols} />
        <thead onContextMenu={passCols.openMenu} onDoubleClick={(e) => resetOnTitle(e, passCols.reset)}>
          <tr>
            {passCols.visible.map((c, i) => (
              <MovableTh key={c} cols={passCols} col={c} index={i}>
                {t(COL_LABEL[c])}
              </MovableTh>
            ))}
          </tr>
        </thead>
        <tbody>
          {passes.map((p) => (
            <tr key={p.aos_unix} className={p.aos_unix <= now.time_unix && p.los_unix > now.time_unix ? 'on' : ''}>
              {passCols.visible.map((c) => passCell(c, p, ''))}
            </tr>
          ))}
        </tbody>
      </table>
      <ColumnsMenu cols={passCols} />
      {passes.length === 0 ? <p className="sat-muted">{t('sat.pass.empty')}</p> : null}
    </div>
        </>
      ),
    },
    {
      id: 'all',
      title: t('sat.all.title'),
      actions: (
        <>
    <label>
      <span>{t('sat.pass.hours')}</span>
      <select value={allHours} onChange={(e) => setAllHours(Number(e.target.value))}>
        {[24, 48].map((h) => (
          <option key={h} value={h}>
            {h}
          </option>
        ))}
      </select>
    </label>
    <span className="sat-muted">{t('sat.pass.minEl')}: {passMinEl}°</span>
        </>
      ),
      ...SAT_MINS.all,
      maxRows: SAT_MAX_ROWS.all,
      maxW: SAT_MAX_COLS.all,
      children: (
      <div className="sat-table sat-table--pass sat-table--all">
        <table ref={allCols.tableRef} className="mv-grid sat-grid-table" style={allCols.tableStyle}>
          <MovableColGroup cols={allCols} />
          <thead onContextMenu={allCols.openMenu} onDoubleClick={(e) => resetOnTitle(e, allCols.reset)}>
            <tr>
              {allCols.visible.map((c, i) => (
                <MovableTh key={c} cols={allCols} col={c} index={i}>
                  {t(COL_LABEL[c])}
                </MovableTh>
              ))}
            </tr>
          </thead>
          <tbody>
            {passesAll.map((p) => {
              const running = p.aos_unix <= now.time_unix && p.los_unix > now.time_unix;
              return (
                <tr key={`${p.norad}-${p.aos_unix}`} className={`sat-row-pick${running ? ' on' : ''}`} onClick={() => pick(p.norad)} title={t('sat.all.pick')}>
                  {allCols.visible.map((c) => passCell(c, p, p.name))}
                </tr>
              );
            })}
          </tbody>
        </table>
        <ColumnsMenu cols={allCols} />
        {passesAll.length === 0 ? <p className="sat-muted">{t('sat.pass.empty')}</p> : null}
      </div>
      ),
    },
    {
      id: 'info',
      title: info ? `${t('sat.info.title')} · ${info.name}` : t('sat.info.title'),
      ...SAT_MINS.info,
      maxRows: SAT_MAX_ROWS.info,
      maxW: SAT_MAX_COLS.info,
      children: info ? (
        <>
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
        </>
      ) : (
        <p className="sat-muted">{t('sat.none')}</p>
      ),
    },
    {
      id: 'ctl',
      title: t('sat.ctl.title'),
      ...SAT_MINS.ctl,
      maxRows: SAT_MAX_ROWS.ctl,
      maxW: SAT_MAX_COLS.ctl,
      children: (
        <>
          <div className="sat-radarpanel">
            <SatRadar size={160} path={now.next_path} now={look ? { az: look.az_deg, el: look.el_deg } : null} rotor={s.rotor_on && now.rotor.ready ? { az: now.rotor.az_deg, el: now.rotor.el_deg } : null} />
          </div>
          <div className="sat-ctl-blocks">
            <section className="sat-block">
              <label className="sat-switch">
                <input type="checkbox" checked={s.doppler_on} onChange={(e) => save({ doppler_on: e.target.checked })} />
                <span>{t('sat.ctl.doppler')}</span>
              </label>
              {info && info.transponders.length > 0 ? (
                <>
                  <label className="sat-field sat-field--wide">
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
                    <label className="sat-field sat-field--wide">
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
              <p className={`sat-status ${now.radio.ready ? 'ok' : 'warn'}`}>{now.radio.ready ? t('sat.ctl.radioReady') : t('sat.ctl.radioNot')}</p>
            </section>
            <section className="sat-block">
              <label className="sat-switch">
                <input type="checkbox" checked={s.rotor_on} onChange={(e) => save({ rotor_on: e.target.checked })} />
                <span>{t('sat.ctl.rotor')}</span>
              </label>
              <label className="sat-field sat-field--check">
                <input type="checkbox" checked={s.rotor_preposition} onChange={(e) => save({ rotor_preposition: e.target.checked })} />
                <span>{t('sat.ctl.prepoint')}</span>
              </label>
              <label className="sat-field">
                <span>{t('sat.ctl.rotMinMove')}</span>
                <input type="number" min={0} max={45} step={0.5} title={t('sat.ctl.rotMinMoveTip')} value={s.rotor_min_move_deg} onChange={(e) => save({ rotor_min_move_deg: Math.max(0, Math.min(45, Number(e.target.value) || 0)) })} />
              </label>
              <label className="sat-field">
                <span>{t('sat.ctl.rotOffset')}</span>
                <input type="number" min={-180} max={180} step={1} title={t('sat.ctl.rotOffsetTip')} value={s.rotor_az_offset_deg} onChange={(e) => save({ rotor_az_offset_deg: Math.max(-180, Math.min(180, Number(e.target.value) || 0)) })} />
              </label>
              <label className="sat-field">
                <span>{t('sat.ctl.minEl')}</span>
                <input type="number" min={0} max={60} value={s.min_el_deg} onChange={(e) => save({ min_el_deg: Math.max(0, Math.min(60, Number(e.target.value) || 0)) })} />
              </label>
              {s.rotor_on && now.rotor.ready ? <p className="sat-muted">{t('sat.ctl.rotorTo', { az: Math.round(now.rotor.az_deg), el: Math.round(now.rotor.el_deg) })}</p> : null}
              <p className={`sat-status ${now.rotor.ready ? 'ok' : 'warn'}`}>{now.rotor.ready ? t('sat.ctl.rotorReady') : t('sat.ctl.rotorNot')}</p>
              {now.rotor.error ? <p className="sat-status warn">{now.rotor.error}</p> : null}
            </section>
          </div>
        </>
      ),
    },
  ];

  return (
    <>
      {!snap.observer ? <p className="notice">{t('sat.noObserver')}</p> : null}

      <header className="sat-head" aria-label={t('sat.title')}>
        <h1 className="visually-hidden">{t('sat.title')}</h1>
        <div className="sat-clock" aria-label="UTC" title={`${clock.date} UTC`}>
          <b>{clock.time}</b>
          <span>UTC</span>
        </div>
        <div className="sat-tle">
          <SatAlertsPanel settings={alerts.settings} blocked={alerts.blocked} update={alerts.update} test={alerts.test} />
          <button type="button" className="glass icon-btn sat-icon-btn" disabled={busy} onClick={refreshTle} title={`${t('sat.tle.refresh')} · ${ageText ? t('sat.tle.age', { t: ageText }) : t('sat.tle.none')}`} aria-label={t('sat.tle.refresh')}>
            <Icon name="refresh" />
          </button>
          {narrow ? null : (
            <>
              <button type="button" className={`glass icon-btn sat-icon-btn${stored.locked ? ' on' : ''}`} aria-pressed={stored.locked} onClick={() => setLayout({ ...stored, locked: !stored.locked })} title={stored.locked ? t('sat.dash.unlock') : t('sat.dash.lock')} aria-label={stored.locked ? t('sat.dash.unlock') : t('sat.dash.lock')}>
                <Icon name={stored.locked ? 'lock' : 'unlock'} />
              </button>
              <button type="button" className="glass icon-btn sat-icon-btn" onClick={() => setLayout({ ...stored, items: defaultLayout() })} title={t('sat.dash.reset')} aria-label={t('sat.dash.reset')}>
                <Icon name="layout" />
              </button>
            </>
          )}
          <button type="button" className="glass icon-btn sat-icon-btn" onClick={() => openSettingsWindow('modesat')} title={t('sat.set.title')} aria-label={t('sat.set.title')}>
            <Icon name="gear" />
          </button>
        </div>
      </header>

      <DashGrid widgets={widgets} layout={stored.items} onLayout={(items) => setLayout({ ...stored, items })} locked={stored.locked} narrow={narrow} defaultWidths={defaultWidths} moveLabel={t('sat.dash.move')} resizeLabel={t('sat.dash.resize')} widthLabel={t('sat.dash.width')} />
    </>
  );
}
