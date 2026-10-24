import { useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent, type WheelEvent as ReactWheelEvent } from 'react';
import type { WorkedSquare } from '../api/client';
import { gridToLatLon, squareCorner } from '../lib/maidenhead';
import type { MapStation } from '../lib/mapStations';
import { FULL_VIEW, panBy, project, zoomAt, type View } from '../lib/mapView';
import { worldPaths } from '../lib/worldMap';

interface Props {
  squares: readonly WorkedSquare[];
  stations: readonly MapStation[];
  /** Locatore della propria stazione. */
  homeGrid?: string | null;
  selected?: string | null;
  onSelect(call: string | null): void;
  zoomInLabel: string;
  zoomOutLabel: string;
  resetLabel: string;
  ariaLabel: string;
}

/** Quanto e' piena una griglia: piu' QSO, piu' colore (scala logaritmica, da 25% a 85%). */
export function squareOpacity(count: number): number {
  return 0.25 + 0.6 * Math.min(1, Math.log10(count + 1) / 3);
}

function tagColor(tag: MapStation['tag'], cq: boolean): string {
  if (tag && tag !== 'cq' && tag !== 'auto') return `var(--tag-${tag.replaceAll('_', '-')}, #f2c94c)`;
  return cq ? '#3fd6c8' : '#7aa2ff';
}

/** Planisfero con i quadrati locatore lavorati, le stazioni che si ricevono ora e la propria stazione. Si trascina e si ingrandisce con la rotella. */
export function WorldMap({ squares, stations, homeGrid, selected, onSelect, zoomInLabel, zoomOutLabel, resetLabel, ariaLabel }: Props) {
  const [view, setView] = useState<View>(FULL_VIEW);
  const [width, setWidth] = useState(800);
  const svgRef = useRef<SVGSVGElement | null>(null);
  const drag = useRef<{ x: number; y: number; moved: boolean } | null>(null);
  const world = useMemo(() => worldPaths(), []);

  useEffect(() => {
    const el = svgRef.current;
    if (!el) return;
    const measure = () => setWidth(el.getBoundingClientRect().width || 800);
    measure();
    if (typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // un pixel dello schermo in gradi della mappa: i segni restano della stessa misura a ogni ingrandimento
  const px = view.w / Math.max(1, width);
  const home = homeGrid ? gridToLatLon(homeGrid) : null;

  const fractions = (e: { clientX: number; clientY: number }) => {
    const r = svgRef.current?.getBoundingClientRect();
    if (!r || r.width <= 0 || r.height <= 0) return { fx: 0.5, fy: 0.5 };
    return { fx: (e.clientX - r.left) / r.width, fy: (e.clientY - r.top) / r.height };
  };

  const onWheel = (e: ReactWheelEvent<SVGSVGElement>) => {
    const { fx, fy } = fractions(e);
    setView((v) => zoomAt(v, e.deltaY < 0 ? 0.8 : 1.25, fx, fy));
  };

  const onPointerDown = (e: ReactPointerEvent<SVGSVGElement>) => {
    if (e.button !== 0) return;
    drag.current = { x: e.clientX, y: e.clientY, moved: false };
    e.currentTarget.setPointerCapture(e.pointerId);
  };

  const onPointerMove = (e: ReactPointerEvent<SVGSVGElement>) => {
    const d = drag.current;
    const r = svgRef.current?.getBoundingClientRect();
    if (!d || !r || r.width <= 0) return;
    const dx = (e.clientX - d.x) / r.width;
    const dy = (e.clientY - d.y) / r.height;
    if (Math.abs(e.clientX - d.x) + Math.abs(e.clientY - d.y) > 3) d.moved = true;
    d.x = e.clientX;
    d.y = e.clientY;
    setView((v) => panBy(v, dx, dy));
  };

  const onPointerUp = () => {
    drag.current = null;
  };

  return (
    <div className="worldmap">
      <svg
        ref={svgRef}
        className="worldmap-svg"
        viewBox={`${view.x} ${view.y} ${view.w} ${view.w / 2}`}
        role="img"
        aria-label={ariaLabel}
        onWheel={onWheel}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        onClick={() => {
          // un clic sul fondo (non un trascinamento) toglie la selezione
          if (!drag.current?.moved) onSelect(null);
        }}
      >
        <rect className="worldmap-sea" x={0} y={0} width={360} height={180} />
        <g className="worldmap-grat">
          {[-150, -120, -90, -60, -30, 0, 30, 60, 90, 120, 150].map((lon) => (
            <line key={`m${lon}`} x1={lon + 180} y1={0} x2={lon + 180} y2={180} />
          ))}
          {[-60, -30, 0, 30, 60].map((lat) => (
            <line key={`p${lat}`} x1={0} y1={90 - lat} x2={360} y2={90 - lat} />
          ))}
        </g>
        <path className="worldmap-land" d={world.land} />
        <path className="worldmap-borders" d={world.borders} />
        <g className="worldmap-worked">
          {squares.map((s) => {
            const c = squareCorner(s.grid);
            if (!c) return null;
            const p = project(c.lon, c.lat + 1);
            return (
              <rect key={s.grid} x={p.x} y={p.y} width={2} height={1} fillOpacity={squareOpacity(s.count)}>
                <title>{`${s.grid} · ${s.count} QSO${s.bands.length > 0 ? ` · ${s.bands.join(', ')}` : ''}`}</title>
              </rect>
            );
          })}
        </g>
        <g className="worldmap-stations">
          {stations.map((s) => {
            const pos = gridToLatLon(s.grid);
            if (!pos) return null;
            const p = project(pos.lon, pos.lat);
            const isSelected = selected === s.call;
            return (
              <circle
                key={s.call}
                className={`worldmap-station${isSelected ? ' worldmap-station--selected' : ''}`}
                cx={p.x}
                cy={p.y}
                r={(isSelected ? 7 : 4.5) * px}
                fill={tagColor(s.tag, s.cq)}
                strokeWidth={1.5 * px}
                onClick={(e) => {
                  e.stopPropagation();
                  onSelect(isSelected ? null : s.call);
                }}
              >
                <title>{`${s.call} · ${s.grid} · ${s.snr} dB${s.country ? ` · ${s.country}` : ''}`}</title>
              </circle>
            );
          })}
        </g>
        {home ? (
          <g className="worldmap-home" transform={`translate(${project(home.lon, home.lat).x} ${project(home.lon, home.lat).y}) scale(${px})`}>
            <path d="M0 -9 L2.6 -3 L9 -2.8 L4 1.4 L5.6 8 L0 4.4 L-5.6 8 L-4 1.4 L-9 -2.8 L-2.6 -3 Z" />
          </g>
        ) : null}
      </svg>
      <div className="worldmap-zoom">
        <button type="button" className="glass" aria-label={zoomInLabel} title={zoomInLabel} onClick={() => setView((v) => zoomAt(v, 0.7, 0.5, 0.5))}>
          +
        </button>
        <button type="button" className="glass" aria-label={zoomOutLabel} title={zoomOutLabel} onClick={() => setView((v) => zoomAt(v, 1.4, 0.5, 0.5))}>
          −
        </button>
        <button type="button" className="glass" aria-label={resetLabel} title={resetLabel} onClick={() => setView(FULL_VIEW)}>
          ⌂
        </button>
      </div>
    </div>
  );
}
