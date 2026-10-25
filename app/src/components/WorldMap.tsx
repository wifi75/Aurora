import * as L from 'leaflet';
import 'leaflet/dist/leaflet.css';
import { useEffect, useRef, useState } from 'react';
import { feature } from 'topojson-client';
import type { GeometryCollection, Topology } from 'topojson-specification';
import countries from 'world-atlas/countries-110m.json';
import type { WorkedSquare } from '../api/client';
import { bearingDeg, distanceKm, greatCircle, latLonToGrid } from '../lib/geo';
import { bandColor, tagColorName } from '../lib/mapColors';
import { gridToLatLon, squareCorner } from '../lib/maidenhead';
import type { MapStation } from '../lib/mapStations';
import { nightPolygon } from '../lib/terminator';

/** Una stazione che ci ha ricevuti (PSK Reporter): si disegna con una linea dalla nostra stazione. */
export interface MapReceiver {
  call: string;
  grid: string;
  snr: number | null;
  band: string | null;
  km: number | null;
}

interface Labels {
  zoomIn: string;
  zoomOut: string;
  reset: string;
  aria: string;
  equator: string;
  cancer: string;
  capricorn: string;
  arctic: string;
  antarctic: string;
}

interface Props {
  squares: readonly WorkedSquare[];
  stations: readonly MapStation[];
  receivers?: readonly MapReceiver[];
  /** Locatore della propria stazione. */
  homeGrid?: string | null;
  selected?: string | null;
  onSelect(call: string | null): void;
  /** Sfondo con terreno e confini (tessere di Esri, da internet); spento resta la mappa disegnata dal programma. */
  tiles: boolean;
  /** Zona di notte. */
  night: boolean;
  /** Piu' bassa, per stare dentro una pagina con altro. */
  compact?: boolean;
  labels: Labels;
}

/** Quanto e' piena una griglia: piu' QSO, piu' colore (scala logaritmica, da 25% a 85%). */
export function squareOpacity(count: number): number {
  return 0.25 + 0.6 * Math.min(1, Math.log10(count + 1) / 3);
}

const TILES_URL = 'https://server.arcgisonline.com/ArcGIS/rest/services/NatGeo_World_Map/MapServer/tile/{z}/{y}/{x}';
const ATTRIBUTION = '© Esri — National Geographic, Esri, DeLorme, NAVTEQ, UNEP-WCMC, USGS, NASA, ESA, METI, NRCAN, GEBCO, NOAA, iPC';
const WORLD: L.LatLngBoundsLiteral = [
  [-85, -180],
  [85, 180],
];

function cssColor(variable: string | null, fallback: string): string {
  if (!variable || typeof document === 'undefined') return fallback;
  const value = getComputedStyle(document.documentElement).getPropertyValue(variable).trim();
  return value || fallback;
}

/**
 * Planisfero come quello di GridTracker2: terreno e confini veri, la zona di notte, tropici e circoli polari, i quadrati
 * lavorati in giallo, le stazioni che si ricevono ora, chi ci ha ricevuto con le sue linee e la lettura sotto il cursore.
 * Si trascina e si ingrandisce con la rotella.
 */
export function WorldMap({ squares, stations, receivers = [], homeGrid, selected, onSelect, tiles, night, compact = false, labels }: Props) {
  const box = useRef<HTMLDivElement | null>(null);
  const map = useRef<L.Map | null>(null);
  const layers = useRef<Record<'tiles' | 'lines' | 'worked' | 'stations' | 'receivers' | 'home', L.Layer | null>>({
    tiles: null,
    lines: null,
    worked: null,
    stations: null,
    receivers: null,
    home: null,
  });
  const select = useRef(onSelect);
  select.current = onSelect;
  const home = homeGrid ? gridToLatLon(homeGrid) : null;
  const homeRef = useRef(home);
  homeRef.current = home;
  const [readout, setReadout] = useState('');

  // la mappa si crea una volta sola
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const m = L.map(el, {
      zoomControl: false,
      attributionControl: true,
      minZoom: 1,
      maxZoom: 10,
      // niente animazione dello zoom: con l'animazione di Leaflet i pulsanti +, - e la rotella non cambiavano il livello
      zoomAnimation: false,
      zoomSnap: 0.25,
      zoomDelta: 0.5,
      wheelPxPerZoomLevel: 100,
      maxBounds: WORLD,
      maxBoundsViscosity: 1,
    });
    m.attributionControl.setPrefix(false);
    // sotto le tessere: la terra disegnata dal programma, per quando non c'e' internet o le tessere sono spente
    m.createPane('landPane').style.zIndex = '150';
    m.getPane('landPane')!.style.pointerEvents = 'none';
    const topology = countries as unknown as Topology;
    L.geoJSON(feature(topology, topology.objects.countries as GeometryCollection), {
      pane: 'landPane',
      interactive: false,
      style: { className: 'worldmap-land', weight: 0.6, fillOpacity: 1 },
    }).addTo(m);
    m.on('click', () => select.current(null));
    m.on('mousemove', (e: L.LeafletMouseEvent) => {
      const { lat, lng } = e.latlng;
      const wrapped = ((((lng + 180) % 360) + 360) % 360) - 180;
      const h = homeRef.current;
      const far = h ? ` · ${distanceKm(h, { lat, lon: wrapped })} km · ${bearingDeg(h, { lat, lon: wrapped })}°` : '';
      setReadout(`${lat.toFixed(2)}, ${wrapped.toFixed(2)}${far} · ${latLonToGrid(lat, wrapped)}`);
    });
    m.on('mouseout', () => setReadout(''));
    const fit = () => {
      m.invalidateSize(false);
      const z = m.getBoundsZoom(WORLD, true);
      m.setMinZoom(z);
      if (m.getZoom() < z) m.setZoom(z);
    };
    m.setView([25, 10], 2, { animate: false });
    fit();
    m.setView([25, 10], m.getMinZoom(), { animate: false });
    map.current = m;
    let observer: ResizeObserver | null = null;
    if (typeof ResizeObserver !== 'undefined') {
      observer = new ResizeObserver(fit);
      observer.observe(el);
    }
    return () => {
      observer?.disconnect();
      m.remove();
      map.current = null;
    };
  }, []);

  const replace = (name: keyof typeof layers.current, next: L.Layer | null) => {
    const m = map.current;
    if (!m) return;
    layers.current[name]?.remove();
    layers.current[name] = next;
    next?.addTo(m);
  };

  // sfondo con terreno e confini
  useEffect(() => {
    const m = map.current;
    if (!m) return;
    m.getContainer().classList.toggle('worldmap--tiles', tiles);
    replace('tiles', tiles ? L.tileLayer(TILES_URL, { attribution: ATTRIBUTION, maxNativeZoom: 10, maxZoom: 10, noWrap: true, bounds: WORLD }) : null);
  }, [tiles]);

  // la notte e le linee fisse: tropici e circoli polari
  useEffect(() => {
    const m = map.current;
    if (!m) return;
    const group = L.layerGroup();
    const lines: Array<[number, string, string]> = [
      [0, labels.equator, 'eq'],
      [23.44, labels.cancer, 'tr'],
      [-23.44, labels.capricorn, 'tr'],
      [66.56, labels.arctic, 'pl'],
      [-66.56, labels.antarctic, 'pl'],
    ];
    for (const [lat, name, kind] of lines) {
      L.polyline(
        [
          [lat, -180],
          [lat, 180],
        ],
        { interactive: false, className: `worldmap-parallel worldmap-parallel--${kind}`, weight: 1 },
      )
        .bindTooltip(name, { permanent: true, direction: 'top', className: 'worldmap-parallel-label', offset: [0, -2] })
        .addTo(group);
    }
    replace('lines', group);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [labels.equator, labels.cancer, labels.capricorn, labels.arctic, labels.antarctic]);

  useEffect(() => {
    const m = map.current;
    if (!m) return;
    if (!night) return;
    let polygon: L.Polygon | null = null;
    const draw = () => {
      const ring = nightPolygon(new Date());
      if (polygon) polygon.setLatLngs(ring);
      else polygon = L.polygon(ring, { interactive: false, stroke: false, fillColor: '#050b18', fillOpacity: 0.42, className: 'worldmap-night' }).addTo(m);
    };
    draw();
    const timer = setInterval(draw, 60_000);
    return () => {
      clearInterval(timer);
      polygon?.remove();
    };
  }, [night]);

  // i quadrati lavorati
  useEffect(() => {
    const group = L.layerGroup();
    for (const s of squares) {
      const c = squareCorner(s.grid);
      if (!c) continue;
      L.rectangle(
        [
          [c.lat, c.lon],
          [c.lat + 1, c.lon + 2],
        ],
        { className: 'worldmap-worked-rect', color: '#6b5d00', weight: 0.8, fillColor: '#ffe14a', fillOpacity: squareOpacity(s.count), interactive: true },
      )
        .bindTooltip(`${s.grid} · ${s.count} QSO${s.bands.length > 0 ? ` · ${s.bands.join(', ')}` : ''}`, { sticky: true })
        .addTo(group);
    }
    replace('worked', group);
  }, [squares]);

  // chi ci ha ricevuti: una linea dalla nostra stazione, del colore della banda
  useEffect(() => {
    const group = L.layerGroup();
    if (home) {
      for (const r of receivers) {
        const to = gridToLatLon(r.grid.slice(0, 6)) ?? gridToLatLon(r.grid.slice(0, 4));
        if (!to) continue;
        const color = bandColor(r.band);
        const path = greatCircle(home, to, 24).map((p) => [p.lat, p.lon] as [number, number]);
        L.polyline(path, { interactive: false, className: 'worldmap-link', color, weight: 1.2, opacity: 0.55 }).addTo(group);
        L.circleMarker([to.lat, to.lon], { className: 'worldmap-receiver', radius: 4, color: '#0b1322', weight: 1, fillColor: color, fillOpacity: 0.95 })
          .bindTooltip(`${r.call} · ${r.grid}${r.band ? ` · ${r.band}` : ''}${r.snr !== null ? ` · ${r.snr} dB` : ''}${r.km !== null ? ` · ${r.km} km` : ''}`, { sticky: true })
          .addTo(group);
      }
    }
    replace('receivers', group);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [receivers, home?.lat, home?.lon]);

  // le stazioni che si ricevono ora, la scelta e la linea fino a lei
  useEffect(() => {
    const group = L.layerGroup();
    for (const s of stations) {
      const pos = gridToLatLon(s.grid);
      if (!pos) continue;
      const isSelected = selected === s.call;
      const { variable, fallback } = tagColorName(s.tag, s.cq);
      L.circleMarker([pos.lat, pos.lon], {
        className: `worldmap-station${isSelected ? ' worldmap-station--selected' : ''}`,
        radius: isSelected ? 8 : 5.5,
        color: isSelected ? '#ffffff' : '#0b1322',
        weight: isSelected ? 2.5 : 1.5,
        fillColor: cssColor(variable, fallback),
        fillOpacity: 1,
        bubblingMouseEvents: false,
      })
        .bindTooltip(`${s.call} · ${s.grid} · ${s.snr} dB${s.country ? ` · ${s.country}` : ''}`, { sticky: true })
        .on('click', () => select.current(isSelected ? null : s.call))
        .addTo(group);
      if (isSelected && home) {
        const path = greatCircle(home, pos, 32).map((p) => [p.lat, p.lon] as [number, number]);
        L.polyline(path, { interactive: false, className: 'worldmap-selected-link', color: '#ffe14a', weight: 2, opacity: 0.9 }).addTo(group);
      }
    }
    replace('stations', group);
  }, [stations, selected, home]);

  // la propria stazione
  useEffect(() => {
    replace(
      'home',
      home
        ? L.marker([home.lat, home.lon], {
            interactive: false,
            zIndexOffset: 1000,
            icon: L.divIcon({ className: 'worldmap-home', html: '<span>★</span>', iconSize: [26, 26], iconAnchor: [13, 13] }),
          })
        : null,
    );
  }, [home?.lat, home?.lon]);

  const zoom = (delta: number) => map.current?.setZoom(map.current.getZoom() + delta);
  const reset = () => {
    const m = map.current;
    if (m) m.setView([25, 10], m.getMinZoom());
  };

  return (
    <div className={`worldmap${compact ? ' worldmap--compact' : ''}`}>
      <div ref={box} className="worldmap-view" role="application" aria-label={labels.aria} />
      <div className="worldmap-zoom">
        <button type="button" className="glass" aria-label={labels.zoomIn} title={labels.zoomIn} onClick={() => zoom(1)}>
          +
        </button>
        <button type="button" className="glass" aria-label={labels.zoomOut} title={labels.zoomOut} onClick={() => zoom(-1)}>
          −
        </button>
        <button type="button" className="glass" aria-label={labels.reset} title={labels.reset} onClick={reset}>
          ⌂
        </button>
      </div>
      <div className="worldmap-readout data" aria-hidden="true">
        {readout || ' '}
      </div>
    </div>
  );
}
