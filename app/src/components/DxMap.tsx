import * as L from 'leaflet';
import 'leaflet/dist/leaflet.css';
import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { feature } from 'topojson-client';
import type { GeometryCollection, Topology } from 'topojson-specification';
import countries from 'world-atlas/countries-110m.json';
import { getDxcc } from '../api/client';

/** Un quadrato da colorare: lavorato o confermato, con il testo per il clic. */
export interface DxSquare {
  grid: string;
  confirmed: boolean;
  text: string;
}
import { bearingDeg, distanceKm, greatCircle, latLonToGrid } from '../lib/geo';
import { tagColorName } from '../lib/mapColors';
import { mapStyle } from '../lib/mapStyles';
import { gridToLatLon, squareCorner } from '../lib/maidenhead';
import { HeatLayer } from '../lib/heatLayer';
import { moonPhase, moonPosition, shadowPolygon } from '../lib/moon';
import { isNight, sunPosition } from '../lib/terminator';

/** Una stazione in onda, con l'eta' del suo ultimo messaggio. */
export interface DxStation {
  call: string;
  grid: string;
  snr: number;
  cq: boolean;
  tag: string | null;
  country: string | null;
  /** Secondi dall'ultimo messaggio. */
  age: number;
}

/** Una chiamata in corso fra due stazioni di cui si conosce il quadrato: `from` chiama `to`. */
export interface DxLink {
  from: string;
  to: string;
  fromPos: { lat: number; lon: number };
  toPos: { lat: number; lon: number };
  /** Una delle due posizioni e' il centro del paese (la stazione non ha mandato il quadrato). */
  guess: boolean;
  /** Secondi dal messaggio. */
  age: number;
  /** La chiamata e' per noi. */
  toMe: boolean;
}

/** Chi ci ha ricevuto (PSK Reporter): il suo quadrato e da quanti secondi. */
export interface DxSpot {
  call: string;
  grid: string;
  age: number;
  snr: number | null;
}

/** Che cosa succede ora in un quadrato, come i codici di GridTracker2. */
export type LiveStatus = 'qrz' | 'cqdx' | 'cq' | 'qsx';

/** Un quadrato con traffico in onda: lo stato piu' importante e l'eta' del messaggio piu' recente. */
export interface DxLiveGrid {
  grid: string;
  status: LiveStatus;
  age: number;
  calls: string[];
}

/** Colori dei codici: QSO e QSL dal log, gli altri dal traffico in onda. */
export const LIVE_COLORS: Record<LiveStatus | 'qso' | 'qsl', string> = {
  qso: '#ffe14a',
  qsl: '#2ecc71',
  qsx: '#7aa2ff',
  cq: '#00d2c8',
  cqdx: '#ff8c1a',
  qrz: '#ff3b3b',
};

export interface DxMapSettings {
  style: string;
  /** Sfondo di notte (quando da noi e' buio), se `autoNight`. */
  nightStyle: string;
  autoNight: boolean;
  /** Luminosita' delle tessere, 0.3–1.5. */
  brightness: number;
  /** Colore dietro le tessere (mare e bordi). */
  background: string;
  greyline: boolean;
  /** Quanto e' scura la notte, 0–0.9. */
  nightOpacity: number;
  /** Anche le fasce dei crepuscoli (civile, nautico, astronomico). */
  twilight: boolean;
  moon: boolean;
  arcs: boolean;
  /** Puntini colorati delle stazioni sopra i quadrati. */
  dots: boolean;
  /** Propagazione: mappa di calore e quadratini di chi ci ha ricevuto (PSK Reporter). */
  heat: boolean;
  /** Linee da noi a chi ci ha ricevuto. */
  spotPaths: boolean;
  /** Colore di quelle linee. */
  spotColor: string;
  /** Minuti: gli spot piu' vecchi non contano. */
  spotMaxMin: number;
  /** Linee fra le stazioni che si chiamano (chi chiama chi). */
  links: boolean;
  /** Colore delle linee fra altre stazioni (QSX). */
  linkColor: string;
  /** Colore delle linee di chi chiama noi (QRZ). */
  linkMeColor: string;
  /** Spessore delle linee in pixel. */
  linkWidth: number;
  /** Secondi per un passo del tratteggio: meno = piu' veloce. */
  antSpeed: number;
  /** Animazione delle linee. */
  arcsAnimated: boolean;
  /** Minuti dopo cui una stazione sparisce; sbiadisce prima. */
  decayMin: number;
}

interface Props {
  settings: DxMapSettings;
  stations: readonly DxStation[];
  links: readonly DxLink[];
  spots: readonly DxSpot[];
  live: readonly DxLiveGrid[];
  squares: readonly DxSquare[];
  homeGrid?: string | null;
  labels: { aria: string; you: string; moon: string; square: string; worked: string; never: string; cq: string; itu: string };
}

const WORLD: L.LatLngBoundsLiteral = [
  [-85, -180],
  [85, 180],
];

function cssColor(variable: string | null, fallback: string): string {
  if (!variable) return fallback;
  return getComputedStyle(document.documentElement).getPropertyValue(variable).trim() || fallback;
}

const wrap = (lon: number) => ((((lon + 180) % 360) + 360) % 360) - 180;

/** Mappa DX: sfondi a scelta, linea grigia con i crepuscoli, Luna, stazioni in onda colorate che sbiadiscono, linee ad arco animate. */
export function DxMap({ settings, stations, links, spots, live, squares, homeGrid, labels }: Props) {
  const box = useRef<HTMLDivElement | null>(null);
  const map = useRef<L.Map | null>(null);
  const layers = useRef<Record<string, L.Layer | null>>({});
  const home = homeGrid ? gridToLatLon(homeGrid) : null;
  const homeRef = useRef(home);
  homeRef.current = home;
  const squaresRef = useRef(squares);
  squaresRef.current = squares;
  const labelsRef = useRef(labels);
  labelsRef.current = labels;
  const [now, setNow] = useState(() => Date.now());
  const [readout, setReadout] = useState('');

  // ogni minuto: notte, Luna e sfondo notturno si aggiornano
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 60_000);
    return () => window.clearInterval(timer);
  }, []);

  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const m = L.map(el, { zoomControl: false, attributionControl: true, minZoom: 1, maxZoom: 10, zoomAnimation: false, zoomSnap: 0.25, zoomDelta: 0.5, wheelPxPerZoomLevel: 100, maxBounds: WORLD, maxBoundsViscosity: 1 });
    m.attributionControl.setPrefix(false);
    m.createPane('landPane').style.zIndex = '150';
    m.getPane('landPane')!.style.pointerEvents = 'none';
    const topology = countries as unknown as Topology;
    L.geoJSON(feature(topology, topology.objects.countries as GeometryCollection), { pane: 'landPane', interactive: false, style: { className: 'worldmap-land', weight: 0.6, fillOpacity: 1 } }).addTo(m);
    m.createPane('shadePane').style.zIndex = '350';
    m.getPane('shadePane')!.style.pointerEvents = 'none';
    m.on('mousemove', (e: L.LeafletMouseEvent) => {
      const { lat, lng } = e.latlng;
      const lon = wrap(lng);
      const h = homeRef.current;
      const far = h ? ` · ${distanceKm(h, { lat, lon })} km · ${bearingDeg(h, { lat, lon })}°` : '';
      setReadout(`${latLonToGrid(lat, lon)} · ${lat.toFixed(2)}, ${lon.toFixed(2)}${far}`);
    });
    m.on('mouseout', () => setReadout(''));
    // clic su un punto vuoto: il quadrato, la posizione, distanza e direzione, se e' gia' lavorato
    m.on('click', (e: L.LeafletMouseEvent) => {
      const { lat, lng } = e.latlng;
      const lon = wrap(lng);
      const grid = latLonToGrid(lat, lon).slice(0, 4).toUpperCase();
      const sq = squaresRef.current.find((s) => s.grid === grid);
      const h = homeRef.current;
      const lb = labelsRef.current;
      const lines = [
        `<strong>${lb.square} ${grid}</strong>`,
        `${lat.toFixed(2)}, ${lon.toFixed(2)}`,
        h ? `${distanceKm(h, { lat, lon })} km · ${bearingDeg(h, { lat, lon })}°` : '',
        sq ? sq.text : lb.never,
      ].filter(Boolean);
      L.popup({ className: 'dxmap-popup' }).setLatLng(e.latlng).setContent(lines.join('<br>')).openOn(m);
    });
    const fit = () => {
      if (el.clientWidth < 10 || el.clientHeight < 10) return;
      m.invalidateSize(false);
      const z = m.getBoundsZoom(WORLD, true);
      if (!Number.isFinite(z)) return;
      m.setMinZoom(z);
      const cur = m.getZoom();
      if (!Number.isFinite(cur) || cur > m.getMaxZoom()) m.setView([25, 10], z, { animate: false });
      else if (cur < z) m.setZoom(z);
    };
    m.setView([25, 10], 2, { animate: false });
    fit();
    const start = m.getMinZoom();
    m.setView([25, 10], Number.isFinite(start) && start > 0 ? start : 2, { animate: false });
    map.current = m;
    const ro = new ResizeObserver(fit);
    ro.observe(el);
    return () => {
      ro.disconnect();
      m.remove();
      map.current = null;
    };
  }, []);

  const replace = (name: string, next: L.Layer | null) => {
    const m = map.current;
    if (!m) return;
    layers.current[name]?.remove();
    layers.current[name] = next;
    next?.addTo(m);
  };

  // sfondo: di notte (da noi) quello notturno, se scelto
  const dark = settings.autoNight && home ? isNight(home.lat, home.lon, new Date(now)) : false;
  const style = mapStyle(dark ? settings.nightStyle : settings.style);
  useLayoutEffect(() => {
    const m = map.current;
    if (!m) return;
    const layer = L.tileLayer(style.url, { attribution: style.attribution, maxNativeZoom: style.maxNativeZoom, maxZoom: 10, noWrap: true, bounds: WORLD, ...(style.subdomains ? { subdomains: style.subdomains } : {}) });
    m.getContainer().classList.add('worldmap--tiles');
    m.getContainer().classList.toggle('worldmap--dark-tiles', !!style.dark);
    replace('tiles', layer);
  }, [style.id]);

  // luminosita' e colore di fondo
  useEffect(() => {
    const m = map.current;
    if (!m) return;
    m.getPane('tilePane')!.style.filter = `brightness(${settings.brightness})`;
    m.getContainer().style.background = settings.background;
  }, [settings.brightness, settings.background]);

  // linea grigia: la notte e, se chiesto, le fasce dei crepuscoli
  useEffect(() => {
    if (!settings.greyline) {
      replace('shade', null);
      return;
    }
    const date = new Date(now);
    const group = L.layerGroup();
    const steps: Array<[number, number]> = settings.twilight
      ? [
          [0, 0.45],
          [-6, 0.2],
          [-12, 0.18],
          [-18, 0.17],
        ]
      : [[0, 1]];
    for (const [alt, share] of steps) {
      L.polygon(shadowPolygon(date, alt), { pane: 'shadePane', interactive: false, stroke: alt === 0, color: '#ffd27a', weight: 1, opacity: 0.5, fillColor: '#050b18', fillOpacity: settings.nightOpacity * share }).addTo(group);
    }
    replace('shade', group);
  }, [settings.greyline, settings.twilight, settings.nightOpacity, now]);

  // Sole e Luna
  useEffect(() => {
    const group = L.layerGroup();
    const date = new Date(now);
    if (settings.greyline) {
      const sun = sunPosition(date);
      L.marker([sun.lat, sun.lon], { interactive: false, icon: L.divIcon({ className: 'dxmap-sun', html: '<span>☀</span>', iconSize: [22, 22], iconAnchor: [11, 11] }) }).addTo(group);
    }
    if (settings.moon) {
      const moon = moonPosition(date);
      const phase = moonPhase(date);
      const glyph = ['🌑', '🌒', '🌓', '🌔', '🌕', '🌖', '🌗', '🌘'][Math.round(phase * 8) % 8];
      L.marker([moon.lat, moon.lon], { icon: L.divIcon({ className: 'dxmap-moon', html: `<span>${glyph}</span>`, iconSize: [24, 24], iconAnchor: [12, 12] }) })
        .bindTooltip(`${labels.moon} · ${Math.round((1 - Math.cos(phase * 2 * Math.PI)) * 50)}%`)
        .addTo(group);
    }
    replace('sky', group);
  }, [settings.moon, settings.greyline, now, labels.moon]);

  // stazioni in onda: colore come in Attivita' di banda, sbiadiscono con l'eta', linea ad arco da noi
  useEffect(() => {
    const group = L.layerGroup();
    const limit = settings.decayMin * 60;
    for (const s of stations) {
      if (s.age > limit || !settings.dots) continue;
      const pos = gridToLatLon(s.grid);
      if (!pos) continue;
      const fade = Math.max(0.15, 1 - s.age / limit);
      const { variable, fallback } = tagColorName(s.tag, s.cq);
      const color = cssColor(variable, fallback);
      if (settings.arcs && home) {
        const path = greatCircle(home, pos, 40).map((p) => [p.lat, p.lon] as [number, number]);
        L.polyline(path, { interactive: false, className: `dxmap-arc${settings.arcsAnimated ? ' dxmap-arc--anim' : ''}`, color, weight: 1.6, opacity: 0.75 * fade }).addTo(group);
      }
      const far = home ? ` · ${distanceKm(home, pos)} km · ${bearingDeg(home, pos)}°` : '';
      L.circleMarker([pos.lat, pos.lon], { radius: 5.5, color: '#0b1322', weight: 1.5, fillColor: color, fillOpacity: fade, opacity: fade, bubblingMouseEvents: false })
        .bindTooltip(`${s.call} · ${s.grid} · ${s.snr} dB${s.country ? ` · ${s.country}` : ''}${far}`, { sticky: true })
        .bindPopup(`<strong>${s.call}</strong><br>${s.grid}${s.country ? ` · ${s.country}` : ''}<br>${s.snr} dB${far}`, { className: 'dxmap-popup' })
        // le zone CQ e ITU dal file dei paesi, quando si apre
        .on('popupopen', (ev: L.PopupEvent) => {
          const lb = labelsRef.current;
          getDxcc(s.call)
            .then((d) => ev.popup.setContent(`<strong>${s.call}</strong><br>${s.grid} · ${d.country}${d.continent ? ` (${d.continent})` : ''}<br>${lb.cq} ${d.cq_zone} · ${lb.itu} ${d.itu_zone}<br>${s.snr} dB${far}`))
            .catch(() => undefined);
        })
        .addTo(group);
    }
    replace('stations', group);
  }, [stations, settings.arcs, settings.arcsAnimated, settings.decayMin, settings.dots, home?.lat, home?.lon]);

  // chi chiama chi: una linea fra le due stazioni, dorata se chiamano noi; sbiadisce come le stazioni
  useEffect(() => {
    if (!settings.links) {
      replace('links', null);
      return;
    }
    const group = L.layerGroup();
    const limit = settings.decayMin * 60;
    for (const k of links) {
      if (k.age > limit) continue;
      const a = k.fromPos;
      const b = k.toPos;
      const fade = Math.max(0.15, 1 - k.age / limit);
      const path = greatCircle(a, b, 40).map((p) => [p.lat, p.lon] as [number, number]);
      // come in GridTracker2: tratteggio giallo-verde che scorre verso chi e' chiamato, cerchietto nero vuoto alle due stazioni
      L.polyline(path, { className: `dxmap-link${settings.arcsAnimated ? ' dxmap-link--anim' : ''}`, color: k.toMe ? settings.linkMeColor : settings.linkColor, weight: k.toMe ? settings.linkWidth + 0.6 : settings.linkWidth, opacity: 0.95 * fade })
        .bindTooltip(`${k.from} → ${k.to}`, { sticky: true })
        .addTo(group);
      for (const end of [a, b]) {
        L.circleMarker([end.lat, end.lon], { interactive: false, radius: 3.5, color: '#111', weight: 1.6, opacity: fade, fill: false }).addTo(group);
      }
    }
    replace('links', group);
  }, [links, settings.links, settings.arcsAnimated, settings.decayMin, settings.linkColor, settings.linkMeColor, settings.linkWidth]);

  // velocita' del tratteggio
  useEffect(() => {
    map.current?.getContainer().style.setProperty('--dxmap-ants', `${settings.antSpeed}s`);
  }, [settings.antSpeed]);

  // propagazione: la mappa di calore di chi ci ha ricevuto e il suo quadratino
  useEffect(() => {
    if (!settings.heat) {
      replace('heat', null);
      replace('spots', null);
      return;
    }
    const limit = settings.spotMaxMin * 60;
    const live = spots.filter((p) => p.age <= limit);
    const heat = new HeatLayer();
    replace('heat', heat);
    const points: Array<{ lat: number; lon: number; weight: number }> = [];
    for (const p of live) {
      const pos = gridToLatLon(p.grid.slice(0, 6)) ?? gridToLatLon(p.grid.slice(0, 4));
      if (pos) points.push({ lat: pos.lat, lon: pos.lon, weight: 0.35 });
    }
    heat.setPoints(points);
    const group = L.layerGroup();
    // linee da noi a chi ci ha ricevuto (le «PSK Spot Paths» di GridTracker2): tratteggio che scorre verso il ricevitore, cerchietto nero
    const from = homeRef.current;
    if (settings.spotPaths && from) {
      for (const p of live) {
        const to = gridToLatLon(p.grid.slice(0, 6)) ?? gridToLatLon(p.grid.slice(0, 4));
        if (!to) continue;
        const path = greatCircle(from, to, 40).map((q) => [q.lat, q.lon] as [number, number]);
        L.polyline(path, { interactive: false, className: `dxmap-link${settings.arcsAnimated ? ' dxmap-link--anim' : ''}`, color: settings.spotColor, weight: settings.linkWidth, opacity: 0.9 }).addTo(group);
        L.circleMarker([to.lat, to.lon], { interactive: false, radius: 3.5, color: '#111', weight: 1.6, fill: false }).addTo(group);
      }
    }
    for (const p of live) {
      const c = squareCorner(p.grid.slice(0, 4));
      if (!c) continue;
      L.rectangle([[c.lat + 0.15, c.lon + 0.3], [c.lat + 0.85, c.lon + 1.7]], { color: '#4a4000', weight: 1, opacity: 0.9, fillColor: '#e8d84a', fillOpacity: 0.9 })
        .bindTooltip(`${p.call} · ${p.grid}${p.snr !== null ? ` · ${p.snr} dB` : ''} · ${Math.round(p.age / 60)} min`, { sticky: true })
        .addTo(group);
    }
    replace('spots', group);
  }, [spots, settings.heat, settings.spotMaxMin, settings.spotPaths, settings.spotColor, settings.linkWidth, settings.arcsAnimated, home?.lat, home?.lon]);

  // casa
  useEffect(() => {
    replace('home', home ? L.marker([home.lat, home.lon], { interactive: false, zIndexOffset: 1000, icon: L.divIcon({ className: 'worldmap-home', html: '<span>★</span>', iconSize: [26, 26], iconAnchor: [13, 13] }) }).bindTooltip(labels.you) : null);
  }, [home?.lat, home?.lon]);

  // quadrati: dal log (QSO giallo, QSL verde) e in onda (QSX, CQ, CQDX, QRZ); quando ci sono tutti e due il quadrato si divide sulla
  // diagonale, in alto a sinistra il log e in basso a destra l'onda, come «Logbook & Live» di GridTracker2. L'onda sbiadisce col tempo.
  useEffect(() => {
    const group = L.layerGroup();
    const limit = settings.decayMin * 60;
    const liveNow = new Map(live.filter((g) => g.age <= limit).map((g) => [g.grid, g]));
    const logGrids = new Set(squares.map((s) => s.grid));
    const shape = (c: { lat: number; lon: number }, part: 'all' | 'top' | 'bottom'): Array<[number, number]> => {
      const sw: [number, number] = [c.lat, c.lon];
      const nw: [number, number] = [c.lat + 1, c.lon];
      const ne: [number, number] = [c.lat + 1, c.lon + 2];
      const se: [number, number] = [c.lat, c.lon + 2];
      return part === 'top' ? [sw, nw, ne] : part === 'bottom' ? [sw, ne, se] : [sw, nw, ne, se];
    };
    for (const s of squares) {
      const c = squareCorner(s.grid);
      if (!c) continue;
      const color = s.confirmed ? LIVE_COLORS.qsl : LIVE_COLORS.qso;
      L.polygon(shape(c, liveNow.has(s.grid) ? 'top' : 'all'), { interactive: false, color, weight: 0.5, opacity: 0.6, fillColor: color, fillOpacity: 0.32 }).addTo(group);
    }
    for (const g of liveNow.values()) {
      const c = squareCorner(g.grid);
      if (!c) continue;
      const fade = Math.max(0.15, 1 - g.age / limit);
      const color = LIVE_COLORS[g.status];
      L.polygon(shape(c, logGrids.has(g.grid) ? 'bottom' : 'all'), { color, weight: 1, opacity: 0.8 * fade, fillColor: color, fillOpacity: 0.55 * fade })
        .bindTooltip(`${g.grid} · ${g.status.toUpperCase()} · ${g.calls.join(', ')}`, { sticky: true })
        .addTo(group);
    }
    replace('squares', group);
  }, [squares, live, settings.decayMin]);

  return (
    <div className="worldmap dxmap">
      <div ref={box} className="worldmap-view" role="application" aria-label={labels.aria} />
      <div className="worldmap-readout data" aria-hidden="true">
        {readout || ' '}
      </div>
    </div>
  );
}
