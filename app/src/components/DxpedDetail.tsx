import { useQuery } from '@tanstack/react-query';
import { Component, useEffect, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { getClusterSpots, getDxcc, postFrequency, type Dxpedition } from '../api/client';
import { useT } from '../i18n';
import { flagCode } from '../lib/flags';
import { bearingDeg, distanceKm, latLonToGrid } from '../lib/geo';
import { readMapOptions } from '../lib/mapOptions';
import { mapLabels } from '../pages/MapPage';
import { WorldMap } from './WorldMap';
import { gridToLatLon } from '../lib/maidenhead';
import { senderOf } from '../lib/wishlist';
import { useStation } from '../state/station';

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
  const mapOptions = readMapOptions();
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
            <WorldMap
              squares={[]}
              stations={[{ call: d.call, grid: latLonToGrid(target.lat, target.lon), snr: 0, cq: true, tag: null, country: d.country, id: 0 }]}
              homeGrid={status?.de_grid}
              selected={d.call}
              onSelect={() => undefined}
              tiles={mapOptions.tiles}
              night={mapOptions.night}
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
