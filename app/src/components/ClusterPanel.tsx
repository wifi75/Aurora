import { toast } from '../lib/toast';
import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState, type CSSProperties, type MouseEvent as ReactMouseEvent, memo } from 'react';
import { getClusterSpots, getDxcc, getDxped, getStationInfo, postFrequency, postMode, rotatorGoto, type ClusterSpot } from '../api/client';
import { bearingDeg, gridCenter, type Point } from '../lib/geo';
import { normalizeAz } from '../lib/rotator';
import { CLUSTER_COLUMNS, type ClusterColumn, type ClusterOptions } from '../lib/clusterOptions';
import { CLUSTER_COL_EM, CLUSTER_COLS_KEY } from '../lib/clusterColumns';
import { ColumnsMenu, MovableColGroup, MovableTh, useMovableColumns } from './MovableColumns';
import { dxpedState, today } from './DxpedPanel';
import { inkFor, useDxpedColor } from '../lib/dxpedColor';
import { useT, type Key } from '../i18n';
import { MultiPick } from './MultiPick';
import { SpotDialog } from './digi/SpotDialog';
import { filterSpots, spotAge, toList, type SpotFilters } from '../lib/cluster';
import { formatKhz, formatKhzWhole } from '../lib/format';
import { flagCode } from '../lib/flags';
import { CallsignTip, prefetchCallsigns } from './CallsignTip';
import { CallMenu, callMenuAt, type CallMenuState } from './CallMenu';
import { cleanCall } from '../lib/callsignPage';
import { BAND_NAMES, bandOf } from '../lib/format';
import { replyErrorMessage } from '../lib/replyError';
import { browserStorage } from '../lib/theme';
import { wishReason } from '../lib/wishlist';
import { useStation, useStatusPick } from '../state/station';
import { useWishlist } from '../state/wishlist';
import '../styles/cluster-grid.css';

const KEY = 'aurora-cluster-filters';
const MODES = ['FT8', 'FT4', 'CW', 'SSB', 'RTTY', 'PSK'] as const;

/** Il titolo di ogni colonna e, per alcune, la spiegazione. */
const COL_LABEL: Record<ClusterColumn, Key> = { de: 'cluster.col.by', freq: 'cluster.col.freq', dx: 'cluster.col.call', country: 'cluster.col.country', mode: 'cluster.col.mode', comment: 'cluster.col.comment', time: 'cluster.col.time' };
const COL_TIP: Partial<Record<ClusterColumn, Key>> = { de: 'tip.ui.clusterBy', comment: 'tip.ui.clusterComment' };

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
function ClusterPanelInner({ onSpot, options }: { onSpot?: (spot: ClusterSpot) => Promise<void>; options?: ClusterOptions } = {}) {
  const t = useT();
  // le DXpedition del calendario NG3K in onda oggi: i loro spot si evidenziano (stessa richiesta del pannello DXpedition, gia' in memoria)
  const dxpedQuery = useQuery({ queryKey: ['online-dxped'], queryFn: getDxped, staleTime: 10 * 60_000, retry: false });
  const day = today();
  const dxpedColor = useDxpedColor();
  const dxpedCalls = new Set((dxpedQuery.data?.dxped.items ?? []).filter((d) => dxpedState(d, day, 0) === 'active').map((d) => d.call.toUpperCase()));
  const status = useStatusPick(['dial_hz', 'mode', 'own_continent', 'de_grid', 'dx_call', 'rx_df']);
  const info = useQuery({ queryKey: ['station-info'], queryFn: getStationInfo, retry: false });
  const cluster = useQuery({ queryKey: ['dx-cluster'], queryFn: () => getClusterSpots(), retry: false, refetchInterval: 4000 });
  const [filters, setFilters] = useState<SpotFilters>(readFilters);
  const [spotOpen, setSpotOpen] = useState(false);
  const wishlist = useWishlist((s) => s.list);
  // ordine e larghezze delle colonne: si cambiano trascinando i titoli e restano ricordati su questo dispositivo (la chiave di sempre)
  const cols = useMovableColumns({ storageKey: CLUSTER_COLS_KEY, columns: CLUSTER_COLUMNS, defaultEm: CLUSTER_COL_EM, elastic: 'comment', hidden: options?.hidden });
  const [callMenu, setCallMenu] = useState<CallMenuState | null>(null);
  const closeCallMenu = useCallback(() => setCallMenu(null), []);

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
  const shown = filterSpots(spots, filters, myBand, status?.own_continent).filter((s) => !filters.newOnly || wishReason(wishlist, s.dx_call, s.grid, s.country, s.band) !== null)
    // la rotella: gli spot piu' vecchi dei minuti scelti non si mostrano
    .filter((s) => !options || options.expireMin === 0 || spotAge(s) <= options.expireMin * 60);

  /** Il rotore gira verso lo spot: dal locatore dello spot, altrimenti dal centro del suo paese. */
  const pointRotor = async (spot: ClusterSpot) => {
    const from = status?.de_grid ? gridCenter(status.de_grid) : null;
    if (!from) {
      toast(t('cluster.rotor.noGrid'), 'error');
      return;
    }
    let to: Point | null = spot.grid ? gridCenter(spot.grid) : null;
    if (!to) {
      try {
        const d = await getDxcc(spot.dx_call);
        if (typeof d.lat === 'number' && typeof d.lon === 'number') to = { lat: d.lat, lon: d.lon };
      } catch {
        // senza paese conosciuto si avvisa qui sotto
      }
    }
    if (!to) {
      toast(t('cluster.rotor.noPos', { call: spot.dx_call }), 'error');
      return;
    }
    const az = normalizeAz(bearingDeg(from, to));
    try {
      await rotatorGoto(az);
      toast(t('cluster.rotor.going', { call: spot.dx_call, az: Math.round(az) }), 'info');
    } catch {
      toast(t('digi.run.rotorFail'), 'error');
    }
  };

  const tune = async (spot: ClusterSpot) => {
    // il rotore gira anche se la radio non c'e' o non risponde
    if (options?.rotor) void pointRotor(spot);
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

  // le colonne che si vedono (la rotella delle pagine PSK, RTTY e CW ne puo' nascondere alcune), nell'ordine scelto
  const { visible } = cols;

  /** Tasto destro sul nominativo (DX o chi l'ha segnalato): il menu con «Apri su QRZ.com». */
  const openCallMenu = (e: ReactMouseEvent<HTMLElement>, call: string) => {
    const clean = cleanCall(call);
    if (!clean) return;
    e.preventDefault();
    cols.closeMenu();
    setCallMenu(callMenuAt(e, clean));
  };

  const cell = (c: ClusterColumn, s: ClusterSpot) => {
    switch (c) {
      case 'de':
        return (
          <td key={c} className="data cluster-de cl-c cl-c--de" title={`${spotAge(s)} s`} onContextMenu={(e) => openCallMenu(e, s.spotter)}>
            <CallsignTip call={s.spotter}>{s.spotter}</CallsignTip>
          </td>
        );
      case 'freq':
        return (
          <td key={c} className="cl-c cl-c--freq">
            <span className="cluster-freq data">{formatKhzWhole(s.freq_khz)}</span>
          </td>
        );
      case 'dx':
        return (
          <td key={c} className="cl-c cl-c--dx" onContextMenu={(e) => openCallMenu(e, s.dx_call)}>
            <CallsignTip call={s.dx_call}><strong className="data cluster-dx">{s.dx_call}</strong></CallsignTip>
          </td>
        );
      case 'country': {
        // bandiera e nome nella stessa cella: la colonna si sposta tutta insieme
        const code = flagCode(s.country);
        return (
          <td key={c} className="cluster-country cl-c cl-c--country">
            {code ? <span className={`fi fi-${code} decode-flag`} aria-hidden="true" /> : null}
            {s.country ?? ''}
          </td>
        );
      }
      case 'mode':
        return <td key={c} className="data cluster-mode cl-c cl-c--mode">{s.mode ?? ''}</td>;
      case 'comment':
        return <td key={c} className="cluster-comment cl-c cl-c--comment">{s.comment}</td>;
      case 'time':
        return <td key={c} className="data cluster-utc cl-c cl-c--time">{s.time_utc.slice(0, 2)}:{s.time_utc.slice(2)}</td>;
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
        <button type="button" className="glass chip" title={t('tip.cluster.spot')} onClick={() => setSpotOpen(true)}>
          {t('ctl.spot')}
        </button>
        <input type="search" title={t('tip.ui.search')} aria-label={t('cluster.filter.search')} placeholder={t('cluster.filter.search')} value={filters.search} onChange={(e) => setFilters({ ...filters, search: e.target.value })} />
        <span className={`cluster-link muted${linked === 0 ? ' cluster-link--down' : ''}`}>
          {linked > 0 ? t('cluster.linked', { n: linked, m: servers.length }) : t('cluster.nobody')}
        </span>
      </div>
      {spotOpen ? <SpotDialog call={status?.dx_call ?? ''} dialHz={status?.dial_hz ?? 0} audioHz={status?.rx_df ?? 0} modeName={status?.mode ?? ''} onClose={() => setSpotOpen(false)} /> : null}
      {spots.length === 0 ? (
        <p className="muted cluster-note">{t('cluster.waiting')}</p>
      ) : shown.length === 0 ? (
        <p className="muted cluster-note">{t('cluster.none')}</p>
      ) : (
        <table ref={cols.tableRef} className="cluster-table cluster-grid mv-grid" style={cols.tableStyle}>
          <MovableColGroup cols={cols} />
          <thead onContextMenu={cols.openMenu}>
            <tr>
              {visible.map((c, i) => {
                const tip = COL_TIP[c];
                return (
                  <MovableTh key={c} cols={cols} col={c} index={i} title={tip ? t(tip) : undefined}>
                    {t(COL_LABEL[c])}
                  </MovableTh>
                );
              })}
            </tr>
          </thead>
          <tbody ref={body}>
            {shown.map((s) => {
              const mine = s.band !== null && s.band === myBand;
              const wanted = wishReason(wishlist, s.dx_call, s.grid, s.country, s.band) !== null;
              const dxped = dxpedCalls.has(s.dx_call.toUpperCase());
              return (
                <tr
                  key={s.id}
                  className={`cluster-row${mine ? ' cluster-row--mine' : ''}${wanted ? ' cluster-row--wanted' : ''}${dxped ? ' cluster-row--dxped' : ''}`}
                  data-dxped={dxped || undefined}
                  title={t('tip.cluster.row', { spotter: s.spotter, server: s.server, freq: formatKhz(s.freq_khz) })}
                  onClick={options?.click === 'single' ? () => void tune(s) : undefined}
                  onDoubleClick={options?.click === 'single' ? undefined : () => void tune(s)}
                >
                  {visible.map((c) => cell(c, s))}
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
      <ColumnsMenu cols={cols} />
      <CallMenu menu={callMenu} onClose={closeCallMenu} />
    </div>
  );
}

/** Senza proprieta': si ridisegna solo per i suoi dati, non ogni volta che si ridisegna la Stazione (piu' volte al secondo). */
export const ClusterPanel = memo(ClusterPanelInner);
