import { useQuery } from '@tanstack/react-query';
import { useLayoutEffect, useRef, useState } from 'react';
import { getClusterSpots } from '../api/client';
import { useT } from '../i18n';
import { type WaterfallSpot, spotsOnWaterfall } from '../lib/cluster';
import { formatKhz } from '../lib/format';

interface Props {
  fMinHz: number;
  fMaxHz: number;
  /** Frequenza della radio (Hz): lo spot cade sulla cascata alla sua distanza da questa. */
  dialHz: number | undefined;
  /** Un clic su uno spot: porta la frequenza di ricezione li' (solo ricezione, non trasmette). */
  onPick(hz: number): void;
}

/** Righe di etichette al massimo; gli spot che non ci stanno finiscono nel «+N». */
const MAX_ROWS = 3;
/** Altezza di una riga di etichette e spazio fra le righe (px): devono restare allineati al CSS di `.wf-spot`. */
const ROW_PX = 15;
const ROW_GAP_PX = 2;
/** Spazio lasciato libero in alto per le etichette Rx/Tx della cascata (px). */
const TOP_PX = 20;
/** Spazio orizzontale minimo fra due etichette sulla stessa riga (px). */
const GAP_PX = 4;
/** Padding orizzontale e bordi di un'etichetta (px), oltre al testo. */
const CHIP_EXTRA_PX = 10;
/** Un «+N» assorbe anche gli spot nascosti che cadono entro questa distanza (px). */
const MERGE_PX = 90;

interface Chip {
  spot: WaterfallSpot;
  x: number;
  left: number;
  row: number;
}

interface More {
  x: number;
  left: number;
  row: number;
  spots: WaterfallSpot[];
}

interface Layout {
  chips: Chip[];
  more: More[];
  /** Spot senza etichetta: resta solo la tacca. */
  ticks: { spot: WaterfallSpot; x: number }[];
}

/**
 * Dispone le etichette in pixel sulle righe in alto: ognuna parte dalla sua tacca (o finisce li', vicino al bordo destro),
 * nella prima riga libera; quelle che non trovano posto si raccolgono in un «+N» vicino.
 */
function layoutSpots(spots: readonly WaterfallSpot[], fMinHz: number, fMaxHz: number, width: number, charPx: number, rows: number): Layout {
  const span = Math.max(1, fMaxHz - fMinHz);
  const rowEnd: number[] = [];
  const chips: Chip[] = [];
  const more: More[] = [];
  const ticks: Layout['ticks'] = [];
  const place = (x: number, w: number): { left: number; row: number } | null => {
    // a destra della tacca; se esce dalla cascata, a sinistra
    const left = x + w > width ? Math.max(0, x - w) : x;
    for (let row = 0; row < rows; row++) {
      if (left >= (rowEnd[row] ?? -Infinity) + GAP_PX) {
        rowEnd[row] = left + w;
        return { left, row };
      }
    }
    return null;
  };
  for (const spot of spots) {
    const x = ((spot.hz - fMinHz) / span) * width;
    const slot = rows > 0 ? place(x, spot.call.length * charPx + CHIP_EXTRA_PX) : null;
    if (slot) {
      chips.push({ spot, x, ...slot });
      continue;
    }
    ticks.push({ spot, x });
    const last = more[more.length - 1];
    if (last && x - last.x <= MERGE_PX) {
      last.spots.push(spot);
      continue;
    }
    // il «+N» ha posto per due cifre; se non ci sta neanche lui, va nel «+N» precedente
    const badge = rows > 0 ? place(x, 3 * charPx + CHIP_EXTRA_PX) : null;
    if (badge) more.push({ x, ...badge, spots: [spot] });
    else if (last) last.spots.push(spot);
    else more.push({ x, left: Math.max(0, Math.min(x, width - 3 * charPx - CHIP_EXTRA_PX)), row: Math.max(0, rows - 1), spots: [spot] });
  }
  return { chips, more, ticks };
}

/** Le misure che servono a disporre le etichette: larghezza della cascata, altezza dell'immagine, larghezza di un carattere. */
function useBox(active: boolean) {
  const layer = useRef<HTMLDivElement>(null);
  const probe = useRef<HTMLSpanElement>(null);
  const [box, setBox] = useState({ width: 0, top: 0, height: 0, charPx: 7 });
  useLayoutEffect(() => {
    const el = layer.current;
    const stack = el?.parentElement;
    if (!el || !stack) return;
    const measure = () => {
      const main = stack.querySelector<HTMLElement>('.wf-main');
      const stackRect = stack.getBoundingClientRect();
      const mainRect = main?.getBoundingClientRect();
      const charPx = probe.current ? probe.current.getBoundingClientRect().width / 10 : 0;
      const next = {
        width: stackRect.width,
        top: mainRect ? mainRect.top - stackRect.top : 0,
        height: mainRect ? mainRect.height : stackRect.height,
        charPx: charPx > 0 ? charPx : 7,
      };
      setBox((prev) => (prev.width === next.width && prev.top === next.top && prev.height === next.height && prev.charPx === next.charPx ? prev : next));
    };
    measure();
    if (typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(measure);
    ro.observe(stack);
    const main = stack.querySelector('.wf-main');
    if (main) ro.observe(main);
    return () => ro.disconnect();
  }, [active]);
  return { layer, probe, box };
}

/** Gli spot del DX Cluster sulla cascata, in alto nell'immagine: tacca alla frequenza audio ed etichetta compatta col nominativo. Usa gli stessi spot della finestra Cluster. */
export function SpotMarkers({ fMinHz, fMaxHz, dialHz, onPick }: Props) {
  const t = useT();
  const cluster = useQuery({ queryKey: ['dx-cluster'], queryFn: () => getClusterSpots(), retry: false, refetchInterval: 4000 });
  const config = cluster.data?.config;
  const spots = config?.enabled && dialHz ? spotsOnWaterfall(cluster.data?.spots ?? [], dialHz, fMinHz, fMaxHz) : [];
  const { layer, probe, box } = useBox(spots.length > 0);
  if (spots.length === 0) return null;

  // misure non ancora pronte (prima del disegno): una larghezza di comodo, poi si ridispone
  const width = box.width > 0 ? box.width : 800;
  // cascata bassa: si sale fino al bordo e si tolgono righe, l'etichetta non scende mai sullo spettro
  const roomFull = box.height > 0 ? box.height - TOP_PX : Infinity;
  const top = roomFull >= ROW_PX ? TOP_PX : 2;
  const room = box.height > 0 ? box.height - top - 2 : Infinity;
  const rows = Math.max(0, Math.min(MAX_ROWS, Math.floor((room + ROW_GAP_PX) / (ROW_PX + ROW_GAP_PX))));
  const { chips, more, ticks } = layoutSpots(spots, fMinHz, fMaxHz, width, box.charPx, rows);
  const rowTop = (row: number) => row * (ROW_PX + ROW_GAP_PX);

  const label = (s: WaterfallSpot) => {
    const base = t('wf.spot', { call: s.call, spotter: s.spotter, freq: formatKhz(s.freq_khz) });
    return s.comment.trim() !== '' ? `${base}\n«${s.comment.trim()}»` : base;
  };
  const short = (s: WaterfallSpot) => `${s.call}  ${formatKhz(s.freq_khz)} kHz  de ${s.spotter}${s.comment.trim() !== '' ? `  «${s.comment.trim()}»` : ''}`;

  return (
    <div ref={layer} className="wf-spots" style={{ top: box.top + top }}>
      <span ref={probe} className="wf-spot wf-spot-probe" aria-hidden="true">
        MMMMMMMMMM
      </span>
      {ticks.map(({ spot, x }) => (
        <button
          key={`t-${spot.call}`}
          type="button"
          className="wf-spot-tick"
          style={{ left: x }}
          title={label(spot)}
          aria-label={label(spot)}
          onPointerDown={(e) => e.stopPropagation()}
          onClick={() => onPick(spot.hz)}
        />
      ))}
      {chips.map(({ spot, x, left, row }) => (
        <div key={spot.call} className="wf-spot-wrap">
          {/* tacca sottile nel punto esatto della frequenza, dal bordo alto della zona fino all'etichetta */}
          <span className="wf-spot-stem" aria-hidden="true" style={{ left: x, height: rowTop(row) + ROW_PX }} />
          <button
            type="button"
            className={`wf-spot${left < x ? ' wf-spot--flip' : ''}`}
            style={{ left, top: rowTop(row) }}
            title={label(spot)}
            aria-label={label(spot)}
            onPointerDown={(e) => e.stopPropagation()}
            onClick={() => onPick(spot.hz)}
          >
            {spot.call}
          </button>
        </div>
      ))}
      {more.map((m) => {
        const tip = `${t('wf.spotMore', { n: String(m.spots.length) })}\n${m.spots.map(short).join('\n')}`;
        return (
          <span key={`m-${m.spots[0]?.call ?? m.x}`} className="wf-spot wf-spot--more" style={{ left: m.left, top: rowTop(m.row) }} title={tip} aria-label={tip} role="note">
            +{m.spots.length}
          </span>
        );
      })}
    </div>
  );
}
