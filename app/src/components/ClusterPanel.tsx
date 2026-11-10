import { toast } from '../lib/toast';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useRef, useState, type CSSProperties, memo } from 'react';
import { getClusterSpots, getDxped, getStationInfo, postFrequency, postMode, type ClusterSpot } from '../api/client';
import { dxpedState, today } from './DxpedPanel';
import { inkFor, useDxpedColor } from '../lib/dxpedColor';
import { useT, type Key } from '../i18n';
import { MultiPick } from './MultiPick';
import { filterSpots, spotAge, toList, type SpotFilters } from '../lib/cluster';
import { formatKhz, formatKhzWhole } from '../lib/format';
import { flagCode } from '../lib/flags';
import { CallsignTip, prefetchCallsigns } from './CallsignTip';
import { BAND_NAMES, bandOf } from '../lib/format';
import { replyErrorMessage } from '../lib/replyError';
import { browserStorage } from '../lib/theme';
import { wishReason } from '../lib/wishlist';
import { useStation, useStatusPick } from '../state/station';
import { useWishlist } from '../state/wishlist';

const KEY = 'aurora-cluster-filters';
const MODES = ['FT8', 'FT4', 'CW', 'SSB', 'RTTY', 'PSK'] as const;

/** I filtri banda/modo salvati nel riquadro cluster del PC (li usa anche il telefono). */
export function readFilters(): SpotFilters {
  const defaults: SpotFilters = { bands: [], modes: [], search: '', spotters: [], newOnly: false };
  try {
    const raw = browserStorage()?.getItem(KEY);
    if (!raw) return defaults;
    const p = JSON.parse(raw) as { bands?: unknown; band?: unknown; modes?: unknown; mode?: unknown; spotters?: unknown; newOnly?: unknown };
    // le versioni precedenti salvavano una sola banda e un solo modo
    return { bands: toList(p.bands ?? p.band), modes: toList(p.modes ?? p.mode), search: '', spotters: toList(p.spotters), newOnly: p.newOnly === true };
  } catch {
    return defaults;
  }
}

/**
 * Porta la radio sulla frequenza esatta di uno spot (kHz -> Hz), cambiando prima il modo se lo spot e' FT4 o FT8 e si e' nell'altro:
 * cosi' il cambio di modo non riporta la radio sulla frequenza standard del modo nuovo. La frequenza si vede subito; se il comando
 * fallisce torna quella di prima e l'errore passa a chi chiama.
 */
export async function tuneSpot(spot: ClusterSpot): Promise<void> {
  const hz = Math.round(spot.freq_khz * 1000);
  const before = useStation.getState().status?.dial_hz;
  useStation.getState().patchStatus({ dial_hz: hz });
  try {
    const want = (spot.mode ?? '').toUpperCase();
    const now = (useStation.getState().status?.mode ?? '').toUpperCase();
    if ((want === 'FT4' || want === 'FT8') && want !== now) {
      useStation.getState().patchStatus({ mode: want });
      await postMode(want);
    }
    const done = await postFrequency(hz);
    if (done.dial_hz) useStation.getState().patchStatus({ dial_hz: done.dial_hz });
  } catch (error) {
    if (before !== undefined) useStation.getState().revertStatus(['dial_hz'], { dial_hz: before });
    throw error;
  }
}

/** Gli spot dei DX Cluster collegati, i piu' recenti per primi; un clic porta la radio sulla banda dello spot. */
/** `onSpot`: cosa fare al clic su uno spot al posto di portare la radio sulla sua frequenza (la pagina dei modi testuali la porta con il segnale sul suo tono). */
function ClusterPanelInner({ onSpot }: { onSpot?: (spot: ClusterSpot) => Promise<void> } = {}) {
  const t = useT();
  // le DXpedition del calendario NG3K in onda oggi: i loro spot si evidenziano (stessa richiesta del pannello DXpedition, gia' in memoria)
  const dxpedQuery = useQuery({ queryKey: ['online-dxped'], queryFn: getDxped, staleTime: 10 * 60_000, retry: false });
  const day = today();
  const dxpedColor = useDxpedColor();
  const dxpedCalls = new Set((dxpedQuery.data?.dxped.items ?? []).filter((d) => dxpedState(d, day, 0) === 'active').map((d) => d.call.toUpperCase()));
  const status = useStatusPick(['dial_hz', 'mode', 'own_continent']);
  const info = useQuery({ queryKey: ['station-info'], queryFn: getStationInfo, retry: false });
  const cluster = useQuery({ queryKey: ['dx-cluster'], queryFn: () => getClusterSpots(), retry: false, refetchInterval: 4000 });
  const [filters, setFilters] = useState<SpotFilters>(readFilters);
  const wishlist = useWishlist((s) => s.list);

  const myBand = status && status.dial_hz > 0 ? bandOf(status.dial_hz) : null;
  // tutto quello che si vede nel cluster (DX e chi ha fatto lo spot) si carica in anticipo: al passaggio del mouse il riquadro e' gia' pronto
  useEffect(() => {
    const visible = cluster.data ? filterSpots(cluster.data.spots, filters, myBand, status?.own_continent) : [];
    prefetchCallsigns(visible.flatMap((s) => [s.dx_call, s.spotter]));
  }, [cluster.data, filters, myBand, status?.own_continent]);
  const change = (next: SpotFilters) => {
    setFilters(next);
    try {
      browserStorage()?.setItem(KEY, JSON.stringify({ bands: next.bands, modes: next.modes, spotters: next.spotters ?? [], newOnly: next.newOnly === true }));
    } catch {
      // senza archivio i filtri non si ricordano, ma la finestra funziona lo stesso
    }
  };

  // come Attivita' di banda: lo spot nuovo arriva in cima e la lista torna subito in cima per mostrarlo
  const body = useRef<HTMLTableSectionElement | null>(null);
  const newestSpot = cluster.data?.spots[0]?.id ?? null;
  useEffect(() => {
    for (let el: HTMLElement | null = body.current; el && el !== document.body; el = el.parentElement) {
      const oy = getComputedStyle(el).overflowY;
      if ((oy === 'auto' || oy === 'scroll') && el.scrollHeight > el.clientHeight + 1) {
        el.scrollTop = 0;
        break;
      }
    }
  }, [newestSpot]);
  if (cluster.isError) return <p className="muted cluster-note">{t('set.loadError')}</p>;
  if (!cluster.data) return <p className="muted cluster-note">…</p>;
  const { config, servers, spots } = cluster.data;
  // il nominativo con cui si entra: quello dei dati di stazione (o, con un core vecchio, quello del cluster)
  const callsign = cluster.data.callsign ?? config.callsign;
  if (!config.enabled) {
    return (
      <div className="cluster-note">
        <p className="muted">{t('cluster.off')}</p>
        <a className="glass glass--link" href="#/impostazioni/cluster">
          {t('cluster.settings')}
        </a>
      </div>
    );
  }
  if (callsign === '') {
    return (
      <div className="cluster-note">
        <p className="muted">{t('cluster.noCall')}</p>
        <a className="glass glass--link" href="#/impostazioni/cluster">
          {t('cluster.settings')}
        </a>
      </div>
    );
  }

  const linked = servers.filter((s) => s.state === 'connected').length;
  // «solo nuovi»: gli spot che la lista dei desideri considera nuovi (paese, griglia, banda...)
  const shown = filterSpots(spots, filters, myBand, status?.own_continent).filter((s) => !filters.newOnly || wishReason(wishlist, s.dx_call, s.grid, s.country, s.band) !== null);

  const tune = async (spot: ClusterSpot) => {
    if (!info.data?.rig_control && !info.data?.manual_dial) {
      toast(t('cluster.noRig'), 'error');
      return;
    }
    try {
      // niente riga «Radio portata su…» sopra la tabella: si vede gia' sul display; gli errori in un avviso che sparisce
      await (onSpot ?? tuneSpot)(spot);
    } catch (error) {
      toast(replyErrorMessage(error, t), 'error');
    }
  };

  return (
    <div className="cluster" style={{ '--dxped-bg': dxpedColor, '--dxped-ink': inkFor(dxpedColor) } as CSSProperties}>
      <div className="cluster-bar" title={t('tip.cluster.filters')}>
        <MultiPick
          label={t('cluster.filter.band')}
          allLabel={t('cluster.filter.bandAll')}
          options={[{ value: 'now', label: t('cluster.filter.bandNow') }, ...BAND_NAMES.map((b) => ({ value: b, label: b }))]}
          selected={filters.bands}
          onChange={(bands) => change({ ...filters, bands })}
        />
        <MultiPick
          label={t('cluster.filter.mode')}
          allLabel={t('cluster.filter.modeAll')}
          options={MODES.map((m) => ({ value: m, label: m }))}
          selected={filters.modes}
          onChange={(modes) => change({ ...filters, modes })}
        />
        {/* chi manda lo spot: il proprio continente o quelli scelti (Luc ON7KEC: gli spot degli europei dicono cosa si sente qui) */}
        <MultiPick
          label={t('cluster.filter.spotter')}
          allLabel={t('cluster.filter.spotterAll')}
          options={[{ value: 'mine', label: t('cluster.filter.spotterMine') }, ...['EU', 'NA', 'SA', 'AS', 'AF', 'OC'].map((c) => ({ value: c, label: t(`cont.${c}` as Key) }))]}
          selected={filters.spotters ?? []}
          onChange={(spotters) => change({ ...filters, spotters })}
        />
        <button type="button" className={`glass chip${filters.newOnly ? ' glass--on' : ''}`} aria-pressed={filters.newOnly === true} title={t('tip.cluster.newOnly')} onClick={() => change({ ...filters, newOnly: !filters.newOnly })}>
          {t('cluster.filter.newOnly')}
        </button>
        <input type="search" title={t('tip.ui.search')} aria-label={t('cluster.filter.search')} placeholder={t('cluster.filter.search')} value={filters.search} onChange={(e) => setFilters({ ...filters, search: e.target.value })} />
        <span className={`cluster-link muted${linked === 0 ? ' cluster-link--down' : ''}`}>
          {linked > 0 ? t('cluster.linked', { n: linked, m: servers.length }) : t('cluster.nobody')}
        </span>
      </div>
      {spots.length === 0 ? (
        <p className="muted cluster-note">{t('cluster.waiting')}</p>
      ) : shown.length === 0 ? (
        <p className="muted cluster-note">{t('cluster.none')}</p>
      ) : (
        <table className="cluster-table">
          <thead>
            <tr>
              <th title={t('tip.ui.clusterBy')}>{t('cluster.col.by')}</th>
              <th>{t('cluster.col.freq')}</th>
              <th>{t('cluster.col.call')}</th>
              {/* «Paese» parte sopra le bandierine: il titolo sta nella colonna della bandiera e sborda nella successiva */}
              {/* il titolo copre bandiera e nome: comincia in linea con la bandiera */}
              <th className="cluster-th-country" colSpan={2}>{t('cluster.col.country')}</th>
              <th>{t('cluster.col.mode')}</th>
              <th title={t('tip.ui.clusterComment')}>{t('cluster.col.comment')}</th>
              <th>{t('cluster.col.time')}</th>
            </tr>
          </thead>
          <tbody ref={body}>
            {shown.map((s) => {
              const code = flagCode(s.country);
              const mine = s.band !== null && s.band === myBand;
              const wanted = wishReason(wishlist, s.dx_call, s.grid, s.country, s.band) !== null;
              const dxped = dxpedCalls.has(s.dx_call.toUpperCase());
              return (
                <tr
                  key={s.id}
                  className={`cluster-row${mine ? ' cluster-row--mine' : ''}${wanted ? ' cluster-row--wanted' : ''}${dxped ? ' cluster-row--dxped' : ''}`}
                  data-dxped={dxped || undefined}
                  title={t('tip.cluster.row', { spotter: s.spotter, server: s.server, freq: formatKhz(s.freq_khz) })}
                  onDoubleClick={() => void tune(s)}
                >
                  <td className="data cluster-de" title={`${spotAge(s)} s`}>
                    <CallsignTip call={s.spotter}>{s.spotter}</CallsignTip>
                  </td>
                  <td>
                    <span className="cluster-freq data">{formatKhzWhole(s.freq_khz)}</span>
                  </td>
                  <td>
                    <CallsignTip call={s.dx_call}><strong className="data cluster-dx">{s.dx_call}</strong></CallsignTip>
                  </td>
                  <td className="cluster-flag">{code ? <span className={`fi fi-${code} decode-flag`} aria-hidden="true" /> : null}</td>
                  <td className="cluster-country">{s.country ?? ''}</td>
                  <td className="data cluster-mode">{s.mode ?? ''}</td>
                  <td className="cluster-comment">{s.comment}</td>
                  <td className="data cluster-utc">{s.time_utc.slice(0, 2)}:{s.time_utc.slice(2)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}

/** Senza proprieta': si ridisegna solo per i suoi dati, non ogni volta che si ridisegna la Stazione (piu' volte al secondo). */
export const ClusterPanel = memo(ClusterPanelInner);
