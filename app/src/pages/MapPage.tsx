import { useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { getWorkedMap } from '../api/client';
import { WorldMap } from '../components/WorldMap';
import { useT } from '../i18n';
import { stationsWithGrid } from '../lib/mapStations';
import { bandOf } from '../lib/format';
import { useStation } from '../state/station';

/** Mappa: i quadrati locatore gia' lavorati (dal log) e le stazioni che si ricevono ora (dal loro locatore). */
export function MapPage() {
  const t = useT();
  const decodes = useStation((s) => s.decodes);
  const status = useStation((s) => s.status);
  const worked = useQuery({ queryKey: ['map-worked'], queryFn: getWorkedMap, retry: false, staleTime: 60_000 });
  const [band, setBand] = useState('all');
  const [showWorked, setShowWorked] = useState(true);
  const [showLive, setShowLive] = useState(true);
  const [selected, setSelected] = useState<string | null>(null);

  const squares = worked.data?.squares ?? [];
  const bands = useMemo(() => [...new Set(squares.flatMap((s) => s.bands))].sort((a, b) => Number.parseInt(a, 10) - Number.parseInt(b, 10)), [squares]);
  const shownSquares = useMemo(() => (band === 'all' ? squares : squares.filter((s) => s.bands.includes(band))), [squares, band]);
  const stations = useMemo(() => stationsWithGrid(decodes), [decodes]);
  const totalQsos = useMemo(() => squares.reduce((sum, s) => sum + s.count, 0), [squares]);
  const picked = stations.find((s) => s.call === selected) ?? null;
  const currentBand = status && status.dial_hz > 0 ? bandOf(status.dial_hz) : null;

  return (
    <div className="map-page">
      <h1 className="page-title">{t('map.title')}</h1>
      <div className="map-bar" title={t('tip.map')}>
        <label className="map-check">
          <input type="checkbox" checked={showWorked} onChange={(e) => setShowWorked(e.target.checked)} />
          <span className="map-swatch map-swatch--worked" aria-hidden="true" />
          {t('map.worked')}
        </label>
        <label className="map-check">
          <input type="checkbox" checked={showLive} onChange={(e) => setShowLive(e.target.checked)} />
          <span className="map-swatch map-swatch--cq" aria-hidden="true" />
          {t('map.live')}
        </label>
        <label className="map-select">
          {t('map.band')}
          <select value={band} onChange={(e) => setBand(e.target.value)} disabled={bands.length === 0}>
            <option value="all">{t('map.allBands')}</option>
            {bands.map((b) => (
              <option key={b} value={b}>
                {b}
              </option>
            ))}
          </select>
        </label>
        <span className="muted map-counts">
          {worked.isPending ? t('map.loading') : worked.isError ? t('map.noLog') : t('map.counts', { squares: squares.length, qsos: totalQsos })}
          {' · '}
          {t('map.liveCount', { n: stations.length })}
          {currentBand ? ` · ${currentBand}` : ''}
        </span>
      </div>
      <WorldMap
        squares={showWorked ? shownSquares : []}
        stations={showLive ? stations : []}
        homeGrid={status?.de_grid}
        selected={selected}
        onSelect={setSelected}
        zoomInLabel={t('map.zoomIn')}
        zoomOutLabel={t('map.zoomOut')}
        resetLabel={t('map.reset')}
        ariaLabel={t('map.aria')}
      />
      <div className="map-legend">
        <span>
          <i className="map-dot map-dot--cq" /> {t('map.legend.cq')}
        </span>
        <span>
          <i className="map-dot map-dot--qso" /> {t('map.legend.qso')}
        </span>
        <span>
          <i className="map-star" aria-hidden="true">★</i> {t('map.legend.home')}
        </span>
        {picked ? <strong className="map-picked data">{t('map.selected', { call: picked.call, grid: picked.grid, snr: picked.snr })}{picked.country ? ` · ${picked.country}` : ''}</strong> : null}
      </div>
    </div>
  );
}
