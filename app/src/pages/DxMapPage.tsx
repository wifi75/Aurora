import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import { getWorkedMap } from '../api/client';
import { DetachButton } from '../components/DetachButton';
import { DetachedNote } from '../components/DetachedNote';
import { DxMap, type DxMapSettings, type DxSquare, type DxStation } from '../components/DxMap';
import { useT } from '../i18n';
import { useGroups } from '../lib/detached';
import { MAP_STYLES } from '../lib/mapStyles';
import { stationsWithGrid } from '../lib/mapStations';
import { browserStorage } from '../lib/theme';
import { useStation } from '../state/station';

const KEY = 'aurora-mappa-dx';
const DEFAULTS: DxMapSettings = {
  style: 'natgeo',
  nightStyle: 'citylights',
  autoNight: false,
  brightness: 1,
  background: '#0b1626',
  greyline: true,
  nightOpacity: 0.65,
  twilight: true,
  moon: true,
  arcs: true,
  arcsAnimated: true,
  decayMin: 15,
};

function readSettings(): DxMapSettings {
  try {
    return { ...DEFAULTS, ...(JSON.parse(browserStorage()?.getItem(KEY) ?? '{}') as Partial<DxMapSettings>) };
  } catch {
    return DEFAULTS;
  }
}

/** Mappa DX: le stazioni in onda sul planisfero, alla GridTracker. */
export function DxMapPage({ solo = false }: { solo?: boolean } = {}) {
  const elsewhere = useGroups().some((g) => g.open && g.items.includes('dxmap'));
  if (elsewhere && !solo) return <DetachedNote item="dxmap" />;
  return <DxMapBody solo={solo} />;
}

function DxMapBody({ solo }: { solo: boolean }) {
  const t = useT();
  const decodes = useStation((s) => s.decodes);
  const status = useStation((s) => s.status);
  const worked = useQuery({ queryKey: ['map-worked'], queryFn: getWorkedMap, retry: false, staleTime: 60_000 });
  const [settings, setSettings] = useState<DxMapSettings>(readSettings);
  const [band, setBand] = useState('all');
  const [mode, setMode] = useState('all');
  const [showSquares, setShowSquares] = useState(true);
  const [open, setOpen] = useState(false);
  const [tick, setTick] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setTick(Date.now()), 15_000);
    return () => window.clearInterval(timer);
  }, []);
  const change = (next: Partial<DxMapSettings>) => {
    const merged = { ...settings, ...next };
    try {
      browserStorage()?.setItem(KEY, JSON.stringify(merged));
    } catch {
      // senza archivio vale finche' la pagina resta aperta
    }
    setSettings(merged);
  };

  const stations: DxStation[] = useMemo(() => {
    const times = new Map(decodes.map((d) => [d.id, d.time_ms]));
    return stationsWithGrid(decodes).map((s) => ({ call: s.call, grid: s.grid, snr: s.snr, cq: s.cq, tag: s.tag, country: s.country, age: Math.max(0, (tick - (times.get(s.id) ?? tick)) / 1000) }));
  }, [decodes, tick]);
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
        .filter((s) => band === 'all' || s.bands.includes(band))
        .map((s) => ({ grid: s.grid, confirmed: false, text: `${workedText}: ${s.count} QSO${s.bands.length ? ` · ${s.bands.join(', ')}` : ''}` }));
    }
    const byGrid = new Map<string, { qsos: number; confirmed: number; bands: Set<string>; modes: Set<string> }>();
    for (const c of cells) {
      if ((band !== 'all' && c.band !== band) || (mode !== 'all' && c.mode !== mode)) continue;
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
    <section className={`dxmap-page${solo ? ' map-page--solo' : ''}`}>
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
        <label className="toggle">
          <input type="checkbox" checked={showSquares} onChange={(e) => setShowSquares(e.target.checked)} />
          {t('dxmap.squares')}
        </label>
        <select value={band} onChange={(e) => setBand(e.target.value)} disabled={!showSquares}>
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
        {solo ? null : <DetachButton item="dxmap" />}
      </div>
      {open ? (
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
        settings={settings}
        stations={stations}
        squares={shown}
        homeGrid={status?.de_grid}
        labels={{ aria: t('map.aria'), you: t('dxmap.you'), moon: t('dxmap.moon'), square: t('dxmap.square'), worked: t('dxmap.worked'), never: t('dxmap.never'), cq: t('dxmap.cq'), itu: t('dxmap.itu') }}
      />
    </section>
  );
}
