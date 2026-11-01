import * as L from 'leaflet';
import 'leaflet/dist/leaflet.css';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import { feature } from 'topojson-client';
import type { GeometryCollection, Topology } from 'topojson-specification';
import countries from 'world-atlas/countries-110m.json';
import { getClusterSpots, getDxcc, postFrequency, type Dxpedition } from '../api/client';
import { useT } from '../i18n';
import { flagCode } from '../lib/flags';
import { bearingDeg, distanceKm, greatCircle, type Point } from '../lib/geo';
import { gridToLatLon } from '../lib/maidenhead';
import { senderOf } from '../lib/wishlist';
import { useStation } from '../state/station';

const LAND = feature(countries as unknown as Topology, (countries as unknown as Topology).objects.countries as GeometryCollection);

/** Una mappa senza tile (funziona anche senza internet): terre, linea dal proprio QTH e punto della DXpedition. */
function MiniMap({ home, target, zoom, className }: { home: Point | null; target: Point; zoom: number | null; className: string }) {
  const box = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    if (!box.current) return;
    const map = L.map(box.current, { zoomControl: false, attributionControl: false, dragging: zoom !== null, scrollWheelZoom: false, worldCopyJump: true });
    L.geoJSON(LAND, { style: { color: '#3d7a63', weight: 0.6, fillColor: '#2f6b57', fillOpacity: 0.9 }, interactive: false }).addTo(map);
    if (home && zoom === null) {
      L.polyline(greatCircle(home, target, 64).map((p) => [p.lat, p.lon] as [number, number]), { color: '#ffd166', weight: 2, dashArray: '6 5' }).addTo(map);
      L.circleMarker([home.lat, home.lon], { radius: 5, color: '#20c997', fillColor: '#20c997', fillOpacity: 1 }).addTo(map);
    }
    L.circleMarker([target.lat, target.lon], { radius: zoom === null ? 7 : 9, color: '#ff5d8a', fillColor: '#ff5d8a', fillOpacity: 1, weight: 6, opacity: 0.35 }).addTo(map);
    if (zoom !== null) map.setView([target.lat, target.lon], zoom);
    else if (home) map.fitBounds(L.latLngBounds([home.lat, home.lon], [target.lat, target.lon]).pad(0.35));
    else map.setView([target.lat, target.lon], 2);
    return () => {
      map.remove();
    };
  }, [home?.lat, home?.lon, target.lat, target.lon, zoom]);
  return <div ref={box} className={className} />;
}

/** «2026-11-20» → «20/11». */
const short = (d: string) => `${d.slice(8, 10)}/${d.slice(5, 7)}`;

/**
 * La scheda di una DXpedition (proposta 1): mappa grande con la linea dal proprio QTH e un riquadro ingrandito sul punto, sotto i dati di NG3K,
 * paese e zone da cty.dat, distanza e direzione, l'ultima volta che e' stata sentita e i pulsanti per andarci.
 */
export function DxpedDetail({ d, onClose }: { d: Dxpedition & { state: string }; onClose(): void }) {
  const t = useT();
  const status = useStation((s) => s.status);
  const decodes = useStation((s) => s.decodes);
  const dxcc = useQuery({ queryKey: ['dxcc', d.call], queryFn: () => getDxcc(d.call), retry: false, staleTime: Infinity });
  const cluster = useQuery({ queryKey: ['dx-cluster'], queryFn: () => getClusterSpots(), retry: false });
  const home = status?.de_grid ? gridToLatLon(status.de_grid) : null;
  const info = dxcc.data;
  const target = info && info.lat != null && info.lon != null ? { lat: info.lat, lon: info.lon } : null;
  const heard = decodes.find((m) => m.id >= 0 && senderOf(m.text)?.call.toUpperCase() === d.call.toUpperCase());
  const spot = cluster.data?.spots.find((s) => s.dx_call.toUpperCase() === d.call.toUpperCase());
  const code = flagCode(d.country) ?? flagCode(info?.country);

  useEffect(() => {
    const esc = (e: KeyboardEvent) => e.key === 'Escape' && onClose();
    document.addEventListener('keydown', esc);
    return () => document.removeEventListener('keydown', esc);
  }, [onClose]);

  return (
    <div className="dxd-backdrop" onClick={onClose}>
      <div className="dxd" role="dialog" aria-label={d.call} onClick={(e) => e.stopPropagation()}>
        <button type="button" className="dxd-close" aria-label={t('dxd.close')} onClick={onClose}>
          ×
        </button>
        <div className="dxd-map">
          {target ? (
            <>
              <MiniMap home={home} target={target} zoom={null} className="dxd-map-big" />
              <div className="dxd-inset">
                <MiniMap home={null} target={target} zoom={5} className="dxd-map-small" />
                <span>{d.country}</span>
              </div>
            </>
          ) : (
            <p className="muted dxd-nomap">{dxcc.isPending ? t('dxp.loading') : t('dxd.noPlace')}</p>
          )}
        </div>
        <div className="dxd-body">
          <div className="dxd-head">
            <strong className="data dxd-call">{d.call}</strong>
            <span className={`dxd-pill dxd-pill--${d.state}`}>
              {d.state === 'active' ? t('dxp.until', { date: short(d.end) }) : d.state === 'soon' ? t('dxp.from', { date: short(d.start) }) : t('dxp.ended', { date: short(d.end) })}
            </span>
          </div>
          <div className="dxd-country">
            {code ? <span className={`fi fi-${code}`} aria-hidden="true" /> : null}
            {d.country}
            {info ? <span className="muted"> · {info.continent} · CQ {info.cq_zone} · ITU {info.itu_zone}</span> : null}
          </div>
          <dl className="dxd-facts">
            <dt>{t('dxd.dates')}</dt>
            <dd>
              {short(d.start)} → {short(d.end)}
            </dd>
            <dt>{t('dxd.info')}</dt>
            <dd>{d.info || '—'}</dd>
            <dt>QSL</dt>
            <dd>{d.qsl || '—'}</dd>
            {home && target ? (
              <>
                <dt>{t('dxd.fromYou')}</dt>
                <dd>{t('dxd.distance', { km: Math.round(distanceKm(home, target)).toLocaleString(), deg: Math.round(bearingDeg(home, target)) % 360 })}</dd>
              </>
            ) : null}
            <dt>{t('dxd.heard')}</dt>
            <dd>{heard ? t('dxd.heardAt', { snr: heard.snr, hz: heard.freq_hz }) : t('dxd.notHeard')}</dd>
          </dl>
          <div className="dxd-actions">
            {spot ? (
              <button type="button" className="glass glass--on chip" title={t('tip.dxd.tune')} onClick={() => void postFrequency(Math.round(spot.freq_khz * 1000))}>
                {t('dxd.tune', { khz: spot.freq_khz.toFixed(1) })}
              </button>
            ) : null}
            <a className="glass chip" href={`https://www.qrz.com/db/${encodeURIComponent(d.call)}`} target="_blank" rel="noreferrer">
              QRZ.com
            </a>
            <a className="glass chip" href="https://www.ng3k.com/Misc/adxo.html" target="_blank" rel="noreferrer">
              NG3K
            </a>
          </div>
        </div>
      </div>
    </div>
  );
}
