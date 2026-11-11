import { toast } from '../lib/toast';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useRef, useState, type CSSProperties, type KeyboardEvent as ReactKeyboardEvent, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent, memo } from 'react';
import { createPortal } from 'react-dom';
import { getClusterSpots, getDxcc, getDxped, getStationInfo, postFrequency, postMode, rotatorGoto, type ClusterSpot } from '../api/client';
import { bearingDeg, gridCenter, type Point } from '../lib/geo';
import { normalizeAz } from '../lib/rotator';
import type { ClusterColumn, ClusterOptions } from '../lib/clusterOptions';
import { CLUSTER_COL_EM, clampColumnEm, defaultClusterLayout, placeColumn, readClusterLayout, saveClusterLayout, type ClusterColumnLayout } from '../lib/clusterColumns';
import { dxpedState, today } from './DxpedPanel';
import { inkFor, useDxpedColor } from '../lib/dxpedColor';
import { useT, type Key } from '../i18n';
import { MultiPick } from './MultiPick';
import { SpotDialog } from './digi/SpotDialog';
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
import '../styles/cluster-grid.css';

const KEY = 'aurora-cluster-filters';
const MODES = ['FT8', 'FT4', 'CW', 'SSB', 'RTTY', 'PSK'] as const;

/** Il titolo di ogni colonna e, per alcune, la spiegazione. */
const COL_LABEL: Record<ClusterColumn, Key> = { de: 'cluster.col.by', freq: 'cluster.col.freq', dx: 'cluster.col.call', country: 'cluster.col.country', mode: 'cluster.col.mode', comment: 'cluster.col.comment', time: 'cluster.col.time' };
const COL_TIP: Partial<Record<ClusterColumn, Key>> = { de: 'tip.ui.clusterBy', comment: 'tip.ui.clusterComment' };
/** Quanto si sposta il mouse prima che il clic sul titolo diventi un trascinamento della colonna. */
const DRAG_PX = 5;

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
  // ordine e larghezze delle colonne: si cambiano trascinando i titoli e restano ricordati su questo dispositivo
  const [layout, setLayoutState] = useState<ClusterColumnLayout>(readClusterLayout);
  const [drag, setDrag] = useState<{ col: ClusterColumn; before: ClusterColumn | null } | null>(null);
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const table = useRef<HTMLTableElement | null>(null);
  const menuBox = useRef<HTMLDivElement | null>(null);
  const focusCol = useRef<ClusterColumn | null>(null);
  const stopPointer = useRef<(() => void) | null>(null);
  const setLayout = (update: (l: ClusterColumnLayout) => ClusterColumnLayout) =>
    setLayoutState((l) => {
      const next = update(l);
      saveClusterLayout(next);
      return next;
    });
  // spostata da tastiera la colonna resta quella attiva, anche se React ne sposta il titolo
  useEffect(() => {
    const col = focusCol.current;
    focusCol.current = null;
    if (col) table.current?.querySelector<HTMLElement>(`th[data-col="${col}"]`)?.focus();
  }, [layout.order]);
  // un trascinamento a meta' quando la finestra si chiude non lascia ascoltatori appesi
  useEffect(() => () => stopPointer.current?.(), []);
  useEffect(() => {
    if (!menu) return;
    const out = (e: PointerEvent) => {
      if (!menuBox.current?.contains(e.target as Node)) setMenu(null);
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setMenu(null);
    };
    const close = () => setMenu(null);
    document.addEventListener('pointerdown', out);
    window.addEventListener('keydown', key);
    window.addEventListener('blur', close);
    window.addEventListener('resize', close);
    return () => {
      document.removeEventListener('pointerdown', out);
      window.removeEventListener('keydown', key);
      window.removeEventListener('blur', close);
      window.removeEventListener('resize', close);
    };
  }, [menu]);

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
  const visible = layout.order.filter((c) => !options?.hidden.includes(c));
  const widthOf = (c: ClusterColumn) => layout.widths[c] ?? CLUSTER_COL_EM[c];
  // Commenti senza una larghezza scelta prende lo spazio che avanza (come prima); con tutte le larghezze scelte la tabella e' larga la somma
  const elastic = visible.includes('comment') && layout.widths.comment === undefined;
  const fixedEm = visible.reduce((n, c) => n + (elastic && c === 'comment' ? 0 : widthOf(c)), 0);
  const tableStyle: CSSProperties = elastic ? { minWidth: `${fixedEm + CLUSTER_COL_EM.comment}em` } : { width: `${fixedEm}em` };
  /** Il corpo del carattere della tabella in px: le larghezze sono in em. */
  const emPx = () => parseFloat(getComputedStyle(table.current ?? document.body).fontSize) || 12;
  const setWidth = (c: ClusterColumn, em: number) => setLayout((l) => ({ ...l, widths: { ...l.widths, [c]: clampColumnEm(em) } }));

  /** Trascinando il titolo la colonna si sposta: lasciata a meta' di un altro titolo, va prima o dopo di quello. Esc la annulla. */
  const startMove = (e: ReactPointerEvent<HTMLTableCellElement>, col: ClusterColumn) => {
    if (e.button !== 0) return;
    const row = e.currentTarget.parentElement;
    if (!row) return;
    stopPointer.current?.();
    const startX = e.clientX;
    let active = false;
    let before: ClusterColumn | null = null;
    const slotAt = (x: number): ClusterColumn | null => {
      for (const th of Array.from(row.querySelectorAll<HTMLElement>('th[data-col]'))) {
        const r = th.getBoundingClientRect();
        if (x < r.left + r.width / 2) return th.dataset.col as ClusterColumn;
      }
      return null;
    };
    const move = (m: PointerEvent) => {
      if (!active && Math.abs(m.clientX - startX) < DRAG_PX) return;
      active = true;
      before = slotAt(m.clientX);
      setDrag({ col, before });
    };
    const end = (apply: boolean) => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', drop);
      window.removeEventListener('pointercancel', cancel);
      window.removeEventListener('keydown', esc);
      window.removeEventListener('blur', cancel);
      stopPointer.current = null;
      setDrag(null);
      if (apply && active) {
        const to = before;
        setLayout((l) => ({ ...l, order: placeColumn(l.order, col, to) }));
      }
    };
    const drop = () => end(true);
    const cancel = () => end(false);
    const esc = (k: KeyboardEvent) => {
      if (k.key === 'Escape') end(false);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', drop);
    window.addEventListener('pointercancel', cancel);
    window.addEventListener('keydown', esc);
    window.addEventListener('blur', cancel);
    stopPointer.current = cancel;
  };

  /** Trascinando il bordo destro del titolo la colonna si allarga o si stringe, come in Excel (e come nel log delle pagine PSK). */
  const startResize = (e: ReactPointerEvent<HTMLSpanElement>, col: ClusterColumn) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    stopPointer.current?.();
    const startX = e.clientX;
    // Commenti che prendeva lo spazio che avanza parte dalla larghezza che ha adesso
    const shown = e.currentTarget.parentElement?.getBoundingClientRect().width ?? 0;
    const em = emPx();
    const start = layout.widths[col] ?? (shown > 0 ? shown / em : CLUSTER_COL_EM[col]);
    let last = start;
    const move = (m: PointerEvent) => {
      last = clampColumnEm(start + (m.clientX - startX) / em);
      setLayoutState((l) => ({ ...l, widths: { ...l.widths, [col]: last } }));
    };
    const end = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', end);
      window.removeEventListener('pointercancel', end);
      stopPointer.current = null;
      if (last !== start) setWidth(col, last);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', end);
    window.addEventListener('pointercancel', end);
    stopPointer.current = end;
  };

  /** Doppio clic sul bordo: la colonna prende la larghezza del suo contenuto piu' lungo. */
  const autoFit = (col: ClusterColumn) => {
    const el = table.current;
    const i = visible.indexOf(col);
    if (!el || i < 0) return;
    let max = 0;
    el.querySelectorAll(`tr > :nth-child(${i + 1})`).forEach((cell) => {
      max = Math.max(max, (cell as HTMLElement).scrollWidth);
    });
    if (max > 0) setWidth(col, max / emPx() + 0.3);
  };

  /** Da tastiera, sul titolo: Alt+frecce sposta la colonna, Alt+Maiusc+frecce la allarga o la stringe. */
  const keyMove = (e: ReactKeyboardEvent<HTMLTableCellElement>, col: ClusterColumn) => {
    if (!e.altKey || (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight')) return;
    e.preventDefault();
    const left = e.key === 'ArrowLeft';
    if (e.shiftKey) {
      setWidth(col, widthOf(col) + (left ? -0.5 : 0.5));
      return;
    }
    const i = visible.indexOf(col);
    if (left ? i <= 0 : i < 0 || i >= visible.length - 1) return;
    const before = left ? (visible[i - 1] ?? null) : (visible[i + 2] ?? null);
    focusCol.current = col;
    setLayout((l) => ({ ...l, order: placeColumn(l.order, col, before) }));
  };

  /** Tasto destro (o il tasto Menu) sui titoli: il menu con «Ripristina colonne». */
  const openMenu = (e: ReactMouseEvent<HTMLTableSectionElement>) => {
    e.preventDefault();
    let { clientX: x, clientY: y } = e;
    if (x === 0 && y === 0) {
      const r = (e.target as HTMLElement).getBoundingClientRect();
      x = r.left;
      y = r.bottom;
    }
    setMenu({ x, y });
  };
  const resetColumns = () => {
    setMenu(null);
    setLayout(() => defaultClusterLayout());
  };

  const cell = (c: ClusterColumn, s: ClusterSpot) => {
    switch (c) {
      case 'de':
        return (
          <td key={c} className="data cluster-de cl-c cl-c--de" title={`${spotAge(s)} s`}>
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
          <td key={c} className="cl-c cl-c--dx">
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
        <table ref={table} className="cluster-table cluster-grid" style={tableStyle}>
          <colgroup>
            {visible.map((c) => (
              <col key={c} style={elastic && c === 'comment' ? undefined : { width: `${widthOf(c)}em` }} />
            ))}
          </colgroup>
          <thead onContextMenu={openMenu}>
            <tr>
              {visible.map((c, i) => {
                const tip = COL_TIP[c];
                const dropBefore = drag !== null && drag.before === c && drag.col !== c;
                const dropAfter = drag !== null && drag.before === null && i === visible.length - 1 && drag.col !== c;
                return (
                  <th
                    key={c}
                    data-col={c}
                    scope="col"
                    tabIndex={0}
                    className={`cl-h cl-h--${c}${drag?.col === c ? ' cl-h--dragging' : ''}${dropBefore ? ' cl-h--drop-before' : ''}${dropAfter ? ' cl-h--drop-after' : ''}`}
                    title={tip ? `${t(tip)}\n${t('tip.cluster.colHead')}` : t('tip.cluster.colHead')}
                    onPointerDown={(e) => startMove(e, c)}
                    onKeyDown={(e) => keyMove(e, c)}
                  >
                    {t(COL_LABEL[c])}
                    <span className="cl-grip" role="separator" aria-orientation="vertical" title={t('digi.log.colGrip')} onPointerDown={(e) => startResize(e, c)} onDoubleClick={() => autoFit(c)} />
                  </th>
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
      {menu
        ? createPortal(
            <div ref={menuBox} className="cl-menu" role="menu" style={{ left: menu.x, top: menu.y }}>
              <button type="button" role="menuitem" autoFocus onClick={resetColumns}>
                {t('cluster.cols.reset')}
              </button>
            </div>,
            document.body,
          )
        : null}
    </div>
  );
}

/** Senza proprieta': si ridisegna solo per i suoi dati, non ogni volta che si ridisegna la Stazione (piu' volte al secondo). */
export const ClusterPanel = memo(ClusterPanelInner);
