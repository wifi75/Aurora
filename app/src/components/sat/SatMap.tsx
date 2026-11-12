import { useEffect, useRef, useState } from 'react';
import { geoEquirectangular, geoPath } from 'd3-geo';
import { feature } from 'topojson-client';
import type { FeatureCollection } from 'geojson';
import type { GeometryCollection, Topology } from 'topojson-specification';
import type { SatPosition } from '../../api/client';
import { footprintPolygon, nightAlpha, project, splitAtAntimeridian, sunAltitudeDeg, type LatLon } from '../../lib/satGeo';

interface Props {
  /** L'immagine della mappa del mondo (equirettangolare); senza, la mappa inclusa in Aurora (terre e confini di Natural Earth). */
  mapUrl: string | null;
  sun: { lat: number; lon: number };
  moon: { lat: number; lon: number };
  observer: { lat: number; lon: number } | null;
  stationLabel: string;
  positions: readonly SatPosition[];
  selected: number | null;
  /** La traccia a terra del satellite scelto: meta' passata, meta' futura. */
  track: readonly LatLon[];
  trackNowIndex: number;
  onSelect(norad: number): void;
}

let world: Promise<FeatureCollection> | null = null;

/** Terre e confini di Natural Earth a 1:50 milioni (`world-atlas`), caricati una volta sola e solo quando serve la mappa inclusa. */
function loadWorld(): Promise<FeatureCollection> {
  world ??= import('world-atlas/countries-50m.json').then((m) => {
    const topo = m.default as unknown as Topology;
    return feature(topo, topo.objects.countries as GeometryCollection) as FeatureCollection;
  });
  return world;
}

/** La mappa inclusa disegnata una volta per ogni larghezza: mare scuro, terre e confini (proiezione equirettangolare come le mappe in immagine). */
function drawBuiltin(countries: FeatureCollection, w: number, h: number, dpr: number): HTMLCanvasElement {
  const c = document.createElement('canvas');
  c.width = Math.round(w * dpr);
  c.height = Math.round(h * dpr);
  const ctx = c.getContext('2d');
  if (!ctx) return c;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  const sea = ctx.createLinearGradient(0, 0, 0, h);
  sea.addColorStop(0, '#0b1a33');
  sea.addColorStop(1, '#0a2440');
  ctx.fillStyle = sea;
  ctx.fillRect(0, 0, w, h);
  const proj = geoEquirectangular()
    .scale(w / (2 * Math.PI))
    .translate([w / 2, h / 2]);
  const path = geoPath(proj, ctx);
  ctx.beginPath();
  path(countries);
  ctx.fillStyle = '#25483a';
  ctx.fill();
  ctx.strokeStyle = 'rgba(160,210,180,0.35)';
  ctx.lineWidth = 0.6;
  ctx.stroke();
  return c;
}

const PALETTE = { grid: 'rgba(255,255,255,0.16)', label: 'rgba(255,255,255,0.75)', past: 'rgba(255,255,255,0.55)', future: '#ffd84d', foot: '#3ddc84', sat: '#ffffff', sel: '#ffd84d' };

/** La mappa dei satelliti: mondo con giorno e notte, Sole e Luna, la stazione, i satelliti con la loro impronta radio e la traccia del satellite scelto. */
export function SatMap({ mapUrl, sun, moon, observer, stationLabel, positions, selected, track, trackNowIndex, onSelect }: Props) {
  const wrap = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [width, setWidth] = useState(900);
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [countries, setCountries] = useState<FeatureCollection | null>(null);
  const base = useRef<{ w: number; dpr: number; canvas: HTMLCanvasElement } | null>(null);
  const hits = useRef<{ x: number; y: number; norad: number }[]>([]);

  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setWidth(Math.max(320, Math.floor(el.clientWidth))));
    ro.observe(el);
    setWidth(Math.max(320, Math.floor(el.clientWidth)));
    return () => ro.disconnect();
  }, []);

  useEffect(() => {
    if (!mapUrl) {
      setImage(null);
      let alive = true;
      void loadWorld().then(
        (fc) => {
          if (alive) setCountries(fc);
        },
        () => undefined,
      );
      return () => {
        alive = false;
      };
    }
    const img = new Image();
    img.onload = () => setImage(img);
    img.onerror = () => setImage(null);
    img.src = mapUrl;
  }, [mapUrl]);

  useEffect(() => {
    const c = canvas.current;
    const ctx = c?.getContext('2d');
    if (!c || !ctx) return;
    const dpr = Math.min(2, window.devicePixelRatio || 1);
    const w = width;
    const h = Math.round(width / 2);
    c.width = Math.round(w * dpr);
    c.height = Math.round(h * dpr);
    c.style.height = `${h}px`;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    // fondo: la mappa scelta, la mappa inclusa o (mentre si carica) un gradiente scuro
    if (image) ctx.drawImage(image, 0, 0, w, h);
    else if (countries) {
      if (!base.current || base.current.w !== w || base.current.dpr !== dpr) base.current = { w, dpr, canvas: drawBuiltin(countries, w, h, dpr) };
      ctx.drawImage(base.current.canvas, 0, 0, w, h);
    } else {
      const g = ctx.createLinearGradient(0, 0, 0, h);
      g.addColorStop(0, '#0b1a33');
      g.addColorStop(1, '#0a2440');
      ctx.fillStyle = g;
      ctx.fillRect(0, 0, w, h);
    }

    // la notte: un'immagine piccola ingrandita, cosi' il bordo e' morbido
    const nw = 180;
    const nh = 90;
    const night = document.createElement('canvas');
    night.width = nw;
    night.height = nh;
    const nctx = night.getContext('2d');
    if (nctx) {
      const img = nctx.createImageData(nw, nh);
      for (let y = 0; y < nh; y++) {
        for (let x = 0; x < nw; x++) {
          const lat = 90 - ((y + 0.5) / nh) * 180;
          const lon = -180 + ((x + 0.5) / nw) * 360;
          const a = nightAlpha(sunAltitudeDeg(lat, lon, sun.lat, sun.lon), 0.74);
          const o = (y * nw + x) * 4;
          img.data[o] = 4;
          img.data[o + 1] = 10;
          img.data[o + 2] = 38;
          img.data[o + 3] = Math.round(a * 255);
        }
      }
      nctx.putImageData(img, 0, 0);
      ctx.imageSmoothingEnabled = true;
      ctx.drawImage(night, 0, 0, w, h);
    }

    // griglia e scritte sui bordi
    ctx.strokeStyle = PALETTE.grid;
    ctx.fillStyle = PALETTE.label;
    ctx.lineWidth = 1;
    ctx.font = '11px system-ui, sans-serif';
    for (let lon = -150; lon <= 150; lon += 30) {
      const [x] = project(0, lon, w, h);
      ctx.beginPath();
      ctx.moveTo(x + 0.5, 0);
      ctx.lineTo(x + 0.5, h);
      ctx.stroke();
      ctx.textAlign = 'center';
      ctx.fillText(`${Math.abs(lon)}${lon < 0 ? 'W' : lon > 0 ? 'E' : ''}`, x, 12);
    }
    for (let lat = -60; lat <= 60; lat += 30) {
      const [, y] = project(lat, 0, w, h);
      ctx.beginPath();
      ctx.moveTo(0, y + 0.5);
      ctx.lineTo(w, y + 0.5);
      ctx.stroke();
      ctx.textAlign = 'left';
      ctx.fillText(`${Math.abs(lat)}${lat < 0 ? 'S' : lat > 0 ? 'N' : ''}`, 4, y - 3);
    }

    const line = (pts: readonly LatLon[]) => {
      ctx.beginPath();
      pts.forEach((p, i) => {
        const [x, y] = project(p[0], p[1], w, h);
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      });
      ctx.stroke();
    };

    // la traccia a terra: il passato tratteggiato, il futuro pieno
    const past = track.slice(0, Math.max(0, trackNowIndex + 1));
    const future = track.slice(Math.max(0, trackNowIndex));
    ctx.lineWidth = 2;
    ctx.setLineDash([4, 5]);
    ctx.strokeStyle = PALETTE.past;
    for (const seg of splitAtAntimeridian(past)) line(seg);
    ctx.setLineDash([]);
    ctx.strokeStyle = PALETTE.future;
    ctx.shadowColor = 'rgba(255,216,77,0.7)';
    ctx.shadowBlur = 6;
    for (const seg of splitAtAntimeridian(future)) line(seg);
    ctx.shadowBlur = 0;

    // satelliti: impronta radio, punto e nome
    const placed: { x: number; y: number; norad: number }[] = [];
    for (const p of positions) {
      const isSel = p.norad === selected;
      ctx.lineWidth = isSel ? 2 : 1.2;
      ctx.strokeStyle = PALETTE.foot;
      ctx.fillStyle = isSel ? 'rgba(61,220,132,0.12)' : 'rgba(61,220,132,0.05)';
      const poly = footprintPolygon(p.lat_deg, p.lon_deg, p.footprint_km);
      for (const seg of splitAtAntimeridian(poly)) {
        line(seg);
        if (seg.length === poly.length) {
          ctx.beginPath();
          seg.forEach((q, i) => {
            const [x, y] = project(q[0], q[1], w, h);
            if (i === 0) ctx.moveTo(x, y);
            else ctx.lineTo(x, y);
          });
          ctx.closePath();
          ctx.fill();
        }
      }
      const [x, y] = project(p.lat_deg, p.lon_deg, w, h);
      placed.push({ x, y, norad: p.norad });
      ctx.fillStyle = isSel ? PALETTE.sel : PALETTE.sat;
      ctx.shadowColor = isSel ? PALETTE.sel : '#9ad';
      ctx.shadowBlur = isSel ? 12 : 6;
      ctx.beginPath();
      ctx.arc(x, y, isSel ? 5.5 : 4, 0, Math.PI * 2);
      ctx.fill();
      ctx.shadowBlur = 0;
      if (isSel) {
        ctx.strokeStyle = PALETTE.sel;
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        ctx.arc(x, y, 10, 0, Math.PI * 2);
        ctx.stroke();
      }
      ctx.fillStyle = '#fff';
      ctx.font = `${isSel ? 'bold ' : ''}12px system-ui, sans-serif`;
      ctx.textAlign = 'left';
      ctx.shadowColor = '#000';
      ctx.shadowBlur = 4;
      ctx.fillText(p.name, x + 10, y - 8);
      ctx.shadowBlur = 0;
    }
    hits.current = placed;

    // la stazione
    if (observer) {
      const [x, y] = project(observer.lat, observer.lon, w, h);
      ctx.strokeStyle = '#ff6b6b';
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(x - 7, y);
      ctx.lineTo(x + 7, y);
      ctx.moveTo(x, y - 7);
      ctx.lineTo(x, y + 7);
      ctx.stroke();
      ctx.fillStyle = '#ffb3b3';
      ctx.font = '12px system-ui, sans-serif';
      ctx.textAlign = 'left';
      ctx.fillText(stationLabel, x + 9, y + 15);
    }

    // il Sole e la Luna
    const [sx, sy] = project(sun.lat, sun.lon, w, h);
    ctx.fillStyle = '#ffd84d';
    ctx.shadowColor = '#ffb300';
    ctx.shadowBlur = 16;
    ctx.beginPath();
    ctx.arc(sx, sy, 8, 0, Math.PI * 2);
    ctx.fill();
    ctx.shadowBlur = 0;
    ctx.strokeStyle = '#ffd84d';
    ctx.lineWidth = 1.5;
    for (let k = 0; k < 12; k++) {
      const a = (k / 12) * Math.PI * 2;
      ctx.beginPath();
      ctx.moveTo(sx + Math.cos(a) * 11, sy + Math.sin(a) * 11);
      ctx.lineTo(sx + Math.cos(a) * 15, sy + Math.sin(a) * 15);
      ctx.stroke();
    }
    const [mx, my] = project(moon.lat, moon.lon, w, h);
    ctx.fillStyle = '#d9d9d9';
    ctx.shadowColor = '#fff';
    ctx.shadowBlur = 8;
    ctx.beginPath();
    ctx.arc(mx, my, 6, 0, Math.PI * 2);
    ctx.fill();
    ctx.shadowBlur = 0;
  }, [width, image, countries, sun.lat, sun.lon, moon.lat, moon.lon, observer, stationLabel, positions, selected, track, trackNowIndex]);

  return (
    <div className="sat-map" ref={wrap}>
      <canvas
        ref={canvas}
        onClick={(e) => {
          const r = e.currentTarget.getBoundingClientRect();
          const x = e.clientX - r.left;
          const y = e.clientY - r.top;
          let best: { norad: number; d: number } | null = null;
          for (const h of hits.current) {
            const d = Math.hypot(h.x - x, h.y - y);
            if (d <= 16 && (!best || d < best.d)) best = { norad: h.norad, d };
          }
          if (best) onSelect(best.norad);
        }}
      />
    </div>
  );
}
