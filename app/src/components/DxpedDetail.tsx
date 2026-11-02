import { useQuery } from '@tanstack/react-query';
import { Component, useEffect, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { getClusterSpots, getDxcc, postFrequency, type Dxpedition } from '../api/client';
import { useT } from '../i18n';
import { flagCode } from '../lib/flags';
import { bearingDeg, distanceKm, latLonToGrid } from '../lib/geo';
import { mapLabels } from '../pages/MapPage';
import { CountryShape } from './CountryShape';
import { WorldMap } from './WorldMap';
import { gridToLatLon } from '../lib/maidenhead';
import { senderOf } from '../lib/wishlist';
import { useStation } from '../state/station';

/** Cartina chiara con confini e nomi (Esri World Street Map, senza chiave). */
const STREET_TILES = 'https://server.arcgisonline.com/ArcGIS/rest/services/World_Street_Map/MapServer/tile/{z}/{y}/{x}';

/** Giorni interi da oggi a una data «2026-11-20» (negativi se passata). */
function daysTo(date: string): number {
  const end = Date.UTC(Number(date.slice(0, 4)), Number(date.slice(5, 7)) - 1, Number(date.slice(8, 10)));
  const now = new Date();
  return Math.round((end - Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate())) / 86_400_000);
}

/** «2026-11-20» → «20/11». */
const short = (d: string) => `${d.slice(8, 10)}/${d.slice(5, 7)}`;

/**
 * La scheda di una DXpedition (proposta 1): mappa grande con la linea dal proprio QTH e un riquadro ingrandito sul punto, sotto i dati di NG3K,
 * paese e zone da cty.dat, distanza e direzione, l'ultima volta che e' stata sentita e i pulsanti per andarci.
 */
/** La scheda dentro una rete di sicurezza, sopra a tutto (in document.body): un errore la chiude invece di svuotare la pagina. */
export function DxpedDetail(props: { d: Dxpedition & { state: string }; onClose(): void }) {
  return createPortal(
    <SafeCard onError={props.onClose}>
      <DxpedCard {...props} />
    </SafeCard>,
    document.body,
  );
}

class SafeCard extends Component<{ children: ReactNode; onError(): void }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  componentDidCatch(error: unknown) {
    console.error('scheda DXpedition', error);
    this.props.onError();
  }
  render() {
    return this.state.failed ? null : this.props.children;
  }
}

function DxpedCard({ d, onClose }: { d: Dxpedition & { state: string }; onClose(): void }) {
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
        <button type="button" className="dxd-close" aria-label={t('dxd.close')} title={t('dxd.close')} onClick={onClose}>
          ×
        </button>
        <div className="dxd-map">
          {target ? (
            <WorldMap
              squares={[]}
              stations={[{ call: d.call, grid: latLonToGrid(target.lat, target.lon), snr: 0, cq: true, tag: null, country: d.country, id: 0 }]}
              homeGrid={status?.de_grid}
              selected={d.call}
              onSelect={() => undefined}
              tiles
              tileUrl={STREET_TILES}
              parallels={false}
              night={false}
              compact
              labels={mapLabels(t)}
            />
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
          <div className="dxd-state">
            <div className={`dxd-st dxd-st--${d.state}`}>
              <small>{t('dxd.state')}</small>
              <b>{d.state === 'active' ? t('dxd.live') : d.state === 'soon' ? t('dxd.soon') : t('dxd.over')}</b>
            </div>
            <div className={`dxd-st${d.state === 'ended' ? '' : ' dxd-st--warn'}`}>
              <small>{d.state === 'soon' ? t('dxd.startsIn') : t('dxd.left')}</small>
              <b>{d.state === 'ended' ? '—' : t('dxd.days', { n: Math.max(0, d.state === 'soon' ? daysTo(d.start) : daysTo(d.end)) })}</b>
            </div>
            <div className={`dxd-st${heard ? ' dxd-st--active' : ''}`}>
              <small>{t('dxd.heard')}</small>
              <b>{heard ? `${heard.snr} dB` : t('dxd.no')}</b>
            </div>
            <div className="dxd-st">
              <small>{t('dxd.spot')}</small>
              <b className="data">{spot ? spot.freq_khz.toFixed(1) : '—'}</b>
            </div>
          </div>
          <div className="dxd-facts-row">
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
            {target ? <CountryShape lat={target.lat} lon={target.lon} label={d.country} /> : null}
          </div>
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
