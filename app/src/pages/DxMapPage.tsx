import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import { getDxcc, getOnline, getPsk, getWorkedMap } from '../api/client';
import { DetachButton } from '../components/DetachButton';
import { DetachedNote } from '../components/DetachedNote';
import { jumpToQueued } from '../components/TxPanel';
import { DxMap, LIVE_COLORS, type DxMapSettings, type DxLink, type DxLiveGrid, type LiveStatus, type DxSpot, type DxSquare, type DxStation } from '../components/DxMap';
import { useT } from '../i18n';
import { useGroups } from '../lib/detached';
import { MAP_STYLES } from '../lib/mapStyles';
import { stationsWithGrid } from '../lib/mapStations';
import { gridToLatLon } from '../lib/maidenhead';
import { bandOf } from '../lib/format';
import { browserStorage } from '../lib/theme';
import { useStation, useStatusPick } from '../state/station';

const KEY = 'aurora-mappa-dx';
const DAY_MS = 86_400_000;
/** Secondi da un messaggio: l'ora dei messaggi e' in millisecondi dalla mezzanotte UTC, non una data intera (a cavallo della mezzanotte si gira). */
function ageOf(nowMs: number, timeOfDayMs: number): number {
  return (((nowMs % DAY_MS) - timeOfDayMs + DAY_MS) % DAY_MS) / 1000;
}
/** Nominativo → quadrato, da tutti i messaggi sentiti da quando il programma e' aperto. */
const LEARNED_GRIDS = new Map<string, string>();
const DEFAULTS: DxMapSettings = {
  style: 'bluemarble',
  nightStyle: 'citylights',
  autoNight: false,
  brightness: 1,
  background: '#0b1626',
  greyline: true,
  nightOpacity: 0.75,
  twilight: true,
  moon: true,
  arcs: true,
  links: true,
  dots: false,
  linkColor: '#d4ff2a',
  linkMeColor: '#ffb300',
  linkWidth: 1.6,
  antSpeed: 0.9,
  heat: true,
  spotPaths: true,
  spotColor: '#d4ff2a',
  spotMaxMin: 15,
  onlyHeardMe: false,
  callLabels: false,
  arcsAnimated: true,
  decayMin: 15,
};

function readSettings(): DxMapSettings {
  try {
    const saved = JSON.parse(browserStorage()?.getItem(KEY) ?? '{}') as Partial<DxMapSettings> & { v?: number };
    // la notte salvata dalle prime versioni era troppo chiara (quasi non si vedeva): una volta sola si porta al valore nuovo
    if ((saved.v ?? 0) < 2) {
      if (saved.nightOpacity === undefined || saved.nightOpacity < 0.7) saved.nightOpacity = DEFAULTS.nightOpacity;
      saved.v = 2;
      browserStorage()?.setItem(KEY, JSON.stringify(saved));
    }
    return { ...DEFAULTS, ...saved };
  } catch {
    return DEFAULTS;
  }
}

/** Mappa DX: le stazioni in onda sul planisfero, alla GridTracker. */
export function DxMapPage({ solo = false, bare = false, inPanel = false }: { solo?: boolean; /** Nella pagina a blocchi: solo la mappa con le linee e la propagazione, niente comandi, legenda e riquadri. */ bare?: boolean; /** Linguetta «Mappa» del pannello Log: si vede anche con la Mappa DX staccata altrove, senza il pulsante per staccarla. */ inPanel?: boolean } = {}) {
  const elsewhere = useGroups().some((g) => g.open && g.items.includes('dxmap'));
  if (elsewhere && !solo && !inPanel) return <DetachedNote item="dxmap" />;
  return <DxMapBody solo={solo} bare={bare} inPanel={inPanel} />;
}

function DxMapBody({ solo, bare, inPanel = false }: { solo: boolean; bare: boolean; inPanel?: boolean }) {
  const t = useT();
  const allDecodes = useStation((s) => s.decodes);
  const dial = useStation((s) => s.status?.dial_hz ?? null);
  // nel pannello Log la mappa si pulisce a ogni cambio di frequenza: contano solo i messaggi arrivati dopo
  const [since, setSince] = useState<{ dial: number | null; id: number }>({ dial: null, id: 0 });
  useEffect(() => {
    if (!inPanel || dial === since.dial) return;
    const last = useStation.getState().decodes.reduce((m, d) => Math.max(m, d.id), 0);
    setSince({ dial, id: since.dial === null ? 0 : last });
  }, [inPanel, dial, since.dial]);
  const decodes = useMemo(() => (inPanel && since.id > 0 ? allDecodes.filter((d) => d.id > since.id) : allDecodes), [allDecodes, inPanel, since.id]);
  const status = useStatusPick(['de_call', 'de_grid', 'dx_call', 'dx_grid', 'tx_enabled', 'transmitting']);
  const worked = useQuery({ queryKey: ['map-worked'], queryFn: getWorkedMap, retry: false, staleTime: 60_000 });
  const online = useQuery({ queryKey: ['online'], queryFn: getOnline, retry: false });
  const psk = useQuery({ queryKey: ['online-psk'], queryFn: getPsk, retry: false, refetchInterval: 60_000, enabled: online.data?.psk === true });
  const [settings, setSettings] = useState<DxMapSettings>(readSettings);
  // «in uso»: la banda della radio adesso (richiesta di Luc: lavorando sui 15 m si vedono solo quadrati e spot dei 15 m)
  const [band, setBand] = useState('auto');
  const bandNow = dial ? bandOf(dial) : null;
  const effBand = band === 'auto' ? (bandNow ?? 'all') : band;
  const [mode, setMode] = useState('all');
  const [showSquares, setShowSquares] = useState(!bare);
  const [open, setOpen] = useState(false);
  const [tick, setTick] = useState(() => Date.now());
  // centro del paese per chi non manda il quadrato (dal file dei paesi del core, una richiesta per nominativo)
  const [countryPos, setCountryPos] = useState<Record<string, { lat: number; lon: number } | null>>({});
  useEffect(() => {
    const timer = window.setInterval(() => setTick(Date.now()), 15_000);
    return () => window.clearInterval(timer);
  }, []);
  const change = (next: Partial<DxMapSettings>) => {
    const merged = { ...settings, ...next };
    try {
      browserStorage()?.setItem(KEY, JSON.stringify({ ...merged, v: 2 }));
    } catch {
      // senza archivio vale finche' la pagina resta aperta
    }
    setSettings(merged);
  };

  const stations: DxStation[] = useMemo(() => {
    const times = new Map(decodes.map((d) => [d.id, d.time_ms]));
    return stationsWithGrid(decodes).map((s) => ({ call: s.call, grid: s.grid, snr: s.snr, cq: s.cq, tag: s.tag, country: s.country, age: ageOf(tick, times.get(s.id) ?? tick % DAY_MS) }));
  }, [decodes, tick]);
  // chi ci ha ricevuto (PSK Reporter), l'ultimo spot di ogni stazione
  const spots: DxSpot[] = useMemo(() => {
    const last = new Map<string, DxSpot>();
    for (const r of psk.data?.psk.reports ?? []) {
      const age = Math.max(0, tick / 1000 - r.time_s);
      const old = last.get(r.receiver);
      if (!r.locator || (old && old.age <= age)) continue;
      // solo gli spot della banda scelta (o di quella in uso)
      if (effBand !== 'all' && r.freq_hz > 0 && bandOf(r.freq_hz) !== effBand) continue;
      last.set(r.receiver, { call: r.receiver, grid: r.locator, snr: r.snr, age });
    }
    return [...last.values()];
  }, [psk.data, tick, effBand]);
  const liveSpots = spots.filter((p) => p.age <= settings.spotMaxMin * 60).length;
  // chi chiama chi: «K1ABC G0XYZ …» e' G0XYZ che chiama K1ABC; vale l'ultimo messaggio di ogni mittente, se i due quadrati sono noti
  const me = status?.de_call?.toUpperCase() ?? '';
  // i quadrati gia' sentiti restano ricordati finche' il programma e' aperto: le linee si tracciano anche quando l'ultimo messaggio non ha il quadrato
  const grids = useMemo(() => {
    for (const s of stations) LEARNED_GRIDS.set(s.call, s.grid);
    const m = new Map(LEARNED_GRIDS);
    if (me && status?.de_grid) m.set(me, status.de_grid);
    const dx = status?.dx_call?.toUpperCase();
    if (dx && status?.dx_grid) m.set(dx, status.dx_grid);
    return m;
  }, [stations, me, status?.de_grid, status?.dx_call, status?.dx_grid]);
  // le chiamate: «K1ABC G0XYZ …» e' G0XYZ che chiama K1ABC (vale l'ultimo messaggio di ogni mittente)
  const calls = useMemo(() => {
    const clean = (c: string) => c.replace(/[<>]/g, '');
    const byFrom = new Map<string, { from: string; to: string; time: number }>();
    for (const d of [...decodes].sort((a, b) => a.id - b.id)) {
      const tokens = d.text.trim().toUpperCase().split(/\s+/);
      if (tokens.length < 2 || ['CQ', 'QRZ', 'DE'].includes(tokens[0]!)) continue;
      const to = clean(tokens[0]!);
      const from = clean(tokens[1]!);
      if (to === from || !/[0-9]/.test(to) || !/[0-9]/.test(from) || to.length < 3 || from.length < 3) continue;
      byFrom.set(from, { from, to, time: d.time_ms });
    }
    // anche io che chiamo il mio DX (le mie trasmissioni non arrivano fra i messaggi ricevuti): con Abilita Tx acceso o mentre trasmetto
    const dx = status?.dx_call ? clean(status.dx_call.toUpperCase()) : '';
    if (me && dx && dx !== me && (status?.tx_enabled || status?.transmitting)) byFrom.set(me, { from: me, to: dx, time: Date.now() % DAY_MS });
    return [...byFrom.values()];
  }, [decodes, me, status?.dx_call, status?.tx_enabled, status?.transmitting]);
  // per i nominativi senza quadrato si chiede il centro del paese, una volta sola
  useEffect(() => {
    const missing = [...new Set(calls.flatMap((c) => [c.from, c.to]))].filter((c) => !grids.has(c) && !(c in countryPos)).slice(0, 20);
    if (missing.length === 0) return;
    let alive = true;
    void Promise.all(missing.map((c) => getDxcc(c).then((d) => [c, d.lat !== undefined && d.lon !== undefined ? { lat: d.lat, lon: d.lon } : null] as const).catch(() => [c, null] as const))).then((found) => {
      if (alive) setCountryPos((old) => ({ ...old, ...Object.fromEntries(found) }));
    });
    return () => {
      alive = false;
    };
  }, [calls, grids, countryPos]);
  const links: DxLink[] = useMemo(() => {
    const where = (call: string): { pos: { lat: number; lon: number }; guess: boolean } | null => {
      const g = grids.get(call);
      const p = g ? gridToLatLon(g) : null;
      if (p) return { pos: p, guess: false };
      const c = countryPos[call];
      return c ? { pos: c, guess: true } : null;
    };
    const out: DxLink[] = [];
    for (const c of calls) {
      const a = where(c.from);
      const b = where(c.to);
      if (!a || !b) continue;
      out.push({ from: c.from, to: c.to, fromPos: a.pos, toPos: b.pos, guess: a.guess || b.guess, toMe: c.to === me || c.from === me, age: c.from === me ? 0 : ageOf(tick, c.time) });
    }
    return out;
  }, [calls, grids, countryPos, me, tick]);
  // quadrati in onda: QRZ (chiamano me) prima di CQDX, CQ e QSX (traffico fra altri), il messaggio piu' recente per l'eta'
  const live: DxLiveGrid[] = useMemo(() => {
    const rank: Record<LiveStatus, number> = { qrz: 4, cqdx: 3, cq: 2, qsx: 1 };
    // prima l'ultimo messaggio di ogni stazione (chi ha chiamato CQ e ora risponde a qualcuno e' QSX), poi i quadrati
    const latest = new Map<string, { status: LiveStatus; time: number }>();
    const byGrid = new Map<string, DxLiveGrid>();
    const note = (call: string, status: LiveStatus, time: number) => {
      const old = latest.get(call);
      if (!old || time >= old.time) latest.set(call, { status, time });
    };
    const put = (call: string, status: LiveStatus, time: number) => {
      const grid = grids.get(call)?.slice(0, 4).toUpperCase();
      if (!grid || call === me) return;
      const age = ageOf(tick, time);
      const old = byGrid.get(grid);
      if (!old) {
        byGrid.set(grid, { grid, status, age, calls: [call] });
        return;
      }
      if (rank[status] > rank[old.status]) old.status = status;
      old.age = Math.min(old.age, age);
      if (!old.calls.includes(call)) old.calls.push(call);
    };
    for (const d of decodes) {
      const tokens = d.text.trim().toUpperCase().split(/\s+/);
      if (tokens[0] === 'CQ' || tokens[0] === 'QRZ') {
        const dx = tokens.length >= 4 && !/[0-9]/.test(tokens[1]!);
        const call = (dx ? tokens[2] : tokens[1])?.replace(/[<>]/g, '');
        if (call) note(call, dx && tokens[1] === 'DX' ? 'cqdx' : 'cq', d.time_ms);
      } else if (tokens.length >= 2) {
        const to = tokens[0]!.replace(/[<>]/g, '');
        note(tokens[1]!.replace(/[<>]/g, ''), to === me ? 'qrz' : 'qsx', d.time_ms);
      }
    }
    for (const [call, v] of latest) put(call, v.status, v.time);
    return [...byGrid.values()];
  }, [decodes, grids, me, tick]);
  const squares = worked.data?.squares ?? [];
  const bands = useMemo(() => [...new Set(squares.flatMap((s) => s.bands))].sort((a, b) => Number.parseInt(a, 10) - Number.parseInt(b, 10)), [squares]);
  const cells = worked.data?.cells;
  const modes = useMemo(() => [...new Set((cells ?? []).map((c) => c.mode).filter(Boolean))].sort(), [cells]);
  // quadrati della banda e del modo scelti: confermato se almeno un QSO e' confermato
  const shown: DxSquare[] = useMemo(() => {
    if (!showSquares) return [];
    const workedText = t('dxmap.worked');
    const confirmedText = t('dxmap.confirmed');
    if (!cells) {
      return squares
        .filter((s) => effBand === 'all' || s.bands.includes(effBand))
        .map((s) => ({ grid: s.grid, confirmed: false, text: `${workedText}: ${s.count} QSO${s.bands.length ? ` · ${s.bands.join(', ')}` : ''}` }));
    }
    const byGrid = new Map<string, { qsos: number; confirmed: number; bands: Set<string>; modes: Set<string> }>();
    for (const c of cells) {
      if ((effBand !== 'all' && c.band !== effBand) || (mode !== 'all' && c.mode !== mode)) continue;
      const g = byGrid.get(c.grid) ?? { qsos: 0, confirmed: 0, bands: new Set<string>(), modes: new Set<string>() };
      g.qsos += c.qsos;
      g.confirmed += c.confirmed;
      g.bands.add(c.band);
      if (c.mode) g.modes.add(c.mode);
      byGrid.set(c.grid, g);
    }
    return [...byGrid].map(([grid, g]) => ({
      grid,
      confirmed: g.confirmed > 0,
      text: `${g.confirmed > 0 ? confirmedText : workedText}: ${g.qsos} QSO${g.confirmed > 0 ? ` (${g.confirmed} ✓)` : ''} · ${[...g.bands].join(', ')}${g.modes.size ? ` · ${[...g.modes].join(', ')}` : ''}`,
    }));
  }, [squares, cells, band, mode, showSquares, t]);
  const confirmedCount = shown.filter((s) => s.confirmed).length;

  return (
    <section className={`dxmap-page${solo ? ' map-page--solo' : ''}${bare ? ' dxmap-page--bare' : ''}${inPanel ? ' dxmap-page--panel' : ''}`}>
      <div className="dxmap-bar">
        <select className="map-style" value={settings.style} title={t('map.style')} onChange={(e) => change({ style: e.target.value })}>
          {MAP_STYLES.map((s) => (
            <option key={s.id} value={s.id}>
              {s.name}
            </option>
          ))}
        </select>
        <label title={t('dxmap.brightness')}>
          ☼
          <input type="range" min={0.3} max={1.5} step={0.05} value={settings.brightness} onChange={(e) => change({ brightness: Number(e.target.value) })} />
        </label>
        <label className="toggle">
          <input type="checkbox" checked={settings.greyline} onChange={(e) => change({ greyline: e.target.checked })} />
          {t('dxmap.greyline')}
        </label>
        <label className="toggle">
          <input type="checkbox" checked={settings.arcs} onChange={(e) => change({ arcs: e.target.checked })} />
          {t('dxmap.arcs')}
        </label>
        <label className="toggle" title={t('dxmap.heat.tip')}>
          <input type="checkbox" checked={settings.heat} onChange={(e) => change({ heat: e.target.checked })} />
          {t('dxmap.heat')}
        </label>
        <label className="toggle" title={t('dxmap.links.tip')}>
          <input type="checkbox" checked={settings.links} onChange={(e) => change({ links: e.target.checked })} />
          {t('dxmap.links')}
        </label>
        <label className="toggle" title={t('dxmap.onlyHeardMe.tip')}>
          <input type="checkbox" checked={settings.onlyHeardMe} onChange={(e) => change({ onlyHeardMe: e.target.checked })} />
          {t('dxmap.onlyHeardMe')}
        </label>
        <label className="toggle" title={t('dxmap.callLabels.tip')}>
          <input type="checkbox" checked={settings.callLabels} onChange={(e) => change({ callLabels: e.target.checked })} />
          {t('dxmap.callLabels')}
        </label>
        <label className="toggle">
          <input type="checkbox" checked={showSquares} onChange={(e) => setShowSquares(e.target.checked)} />
          {t('dxmap.squares')}
        </label>
        <select value={band} onChange={(e) => setBand(e.target.value)}>
          <option value="auto">{t('dxmap.bandNow')}{bandNow ? ` (${bandNow})` : ''}</option>
          <option value="all">{t('dxmap.allBands')}</option>
          {bands.map((b) => (
            <option key={b} value={b}>
              {b}
            </option>
          ))}
        </select>
        {modes.length > 0 ? (
          <select value={mode} onChange={(e) => setMode(e.target.value)} disabled={!showSquares}>
            <option value="all">{t('dxmap.allModes')}</option>
            {modes.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
        ) : null}
        {showSquares ? (
          <span className="dxmap-legend" title={`${t('dxmap.worked')} / ${t('dxmap.confirmed')}`}>
            <i className="dxmap-key dxmap-key--worked" />
            {shown.length - confirmedCount}
            <i className="dxmap-key dxmap-key--confirmed" />
            {confirmedCount}
          </span>
        ) : null}
        <span className="dxmap-count muted">{t('dxmap.onAir', { n: stations.filter((s) => s.age <= settings.decayMin * 60).length })}</span>
        <button type="button" className="solo-btn dxmap-more" aria-expanded={open} title={t('dxmap.more')} aria-label={t('dxmap.more')} onClick={() => setOpen(!open)}>
          ⚙
        </button>
        {solo || inPanel ? null : <DetachButton item="dxmap" />}
      </div>
      {open && !bare ? (
        <div className="dxmap-bar dxmap-bar--more">
          <label title={t('dxmap.background')}>
            {t('dxmap.background')}
            <input type="color" value={settings.background} onChange={(e) => change({ background: e.target.value })} />
          </label>
          <label className="toggle">
            <input type="checkbox" checked={settings.autoNight} onChange={(e) => change({ autoNight: e.target.checked })} />
            {t('dxmap.autoNight')}
          </label>
          <select className="map-style" value={settings.nightStyle} disabled={!settings.autoNight} onChange={(e) => change({ nightStyle: e.target.value })}>
            {MAP_STYLES.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
          <label>
            {t('dxmap.nightDark')}
            <input type="range" min={0} max={0.9} step={0.05} value={settings.nightOpacity} onChange={(e) => change({ nightOpacity: Number(e.target.value) })} />
          </label>
          <label className="toggle">
            <input type="checkbox" checked={settings.twilight} onChange={(e) => change({ twilight: e.target.checked })} />
            {t('dxmap.twilight')}
          </label>
          <label className="toggle">
            <input type="checkbox" checked={settings.moon} onChange={(e) => change({ moon: e.target.checked })} />
            {t('dxmap.moon')}
          </label>
          <label className="toggle">
            <input type="checkbox" checked={settings.arcsAnimated} onChange={(e) => change({ arcsAnimated: e.target.checked })} />
            {t('dxmap.animated')}
          </label>
          <span className="dxmap-group">
            {t('dxmap.heat')}:
            <label className="toggle">
              <input type="checkbox" checked={settings.spotPaths} onChange={(e) => change({ spotPaths: e.target.checked })} />
              {t('dxmap.spotPaths')}
            </label>
            <label title={t('dxmap.linkColor')}>
              {t('dxmap.linkColor')}
              <input type="color" value={settings.spotColor} onChange={(e) => change({ spotColor: e.target.value })} />
            </label>
          </span>
          <label className="toggle">
            <input type="checkbox" checked={settings.dots} onChange={(e) => change({ dots: e.target.checked })} />
            {t('dxmap.dots')}
          </label>
          <span className="dxmap-group">
            {t('dxmap.links')}:
            <label title={t('dxmap.linkColor')}>
              {t('dxmap.linkColor')}
              <input type="color" value={settings.linkColor} onChange={(e) => change({ linkColor: e.target.value })} />
            </label>
            <label title={t('dxmap.linkMeColor')}>
              {t('dxmap.linkMeColor')}
              <input type="color" value={settings.linkMeColor} onChange={(e) => change({ linkMeColor: e.target.value })} />
            </label>
            <label>
              {t('dxmap.linkWidth')}
              <input type="range" min={0.8} max={4} step={0.2} value={settings.linkWidth} onChange={(e) => change({ linkWidth: Number(e.target.value) })} />
            </label>
            <label>
              {t('dxmap.antSpeed')}
              <input type="range" min={0.2} max={3} step={0.1} value={3.2 - settings.antSpeed} disabled={!settings.arcsAnimated} onChange={(e) => change({ antSpeed: Math.round((3.2 - Number(e.target.value)) * 10) / 10 })} />
            </label>
          </span>
          <label>
            {t('dxmap.decay')}
            <select value={settings.decayMin} onChange={(e) => change({ decayMin: Number(e.target.value) })}>
              {[2, 5, 10, 15, 30, 60].map((m) => (
                <option key={m} value={m}>
                  {m} min
                </option>
              ))}
            </select>
          </label>
        </div>
      ) : null}
      <DxMap
        whole={bare && !inPanel}
        onPick={(call) => jumpToQueued(call, status?.de_call ?? undefined, t)}
        // «solo chi mi sente»: niente linee verso le stazioni sentite ne' fra chi si chiama, restano quelle di PSK Reporter
        settings={settings.onlyHeardMe ? { ...settings, arcs: false, links: false } : bare ? { ...settings, links: false } : settings}
        txTo={status?.transmitting && status.dx_grid ? gridToLatLon(status.dx_grid) ?? undefined : undefined}
        stations={stations}
        links={links}
        spots={spots}
        live={live}
        squares={shown}
        homeGrid={status?.de_grid}
        labels={{ aria: t('map.aria'), you: t('dxmap.you'), moon: t('dxmap.moon'), square: t('dxmap.square'), worked: t('dxmap.worked'), never: t('dxmap.never'), cq: t('dxmap.cq'), itu: t('dxmap.itu') }}
      />
      {bare ? null : <div className="dxmap-legendbox" aria-label={t('dxmap.legend')}>
        {(['qso', 'qsl', 'qsx', 'cq', 'cqdx', 'qrz'] as const).map((k) => (
          <span key={k} title={t(`dxmap.code.${k}`)}>
            <i style={{ background: LIVE_COLORS[k] }} />
            {k.toUpperCase()}
          </span>
        ))}
      </div>}
      {settings.heat && !bare ? (
        <div className="dxmap-spotbox">
          <strong>{t('dxmap.spotRx', { n: liveSpots })}</strong>
          <label>
            {t('dxmap.spotMax')}
            <select value={settings.spotMaxMin} onChange={(e) => change({ spotMaxMin: Number(e.target.value) })}>
              {[5, 15, 30, 60, 120, 360].map((m) => (
                <option key={m} value={m}>
                  {String(Math.floor(m / 60)).padStart(2, '0')}:{String(m % 60).padStart(2, '0')}
                </option>
              ))}
            </select>
          </label>
          {online.data?.psk === false ? <span className="dxmap-spotoff">{t('dxmap.pskOff')}</span> : null}
        </div>
      ) : null}
    </section>
  );
}
