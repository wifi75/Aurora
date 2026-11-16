import { useEffect, useMemo, useRef, useState } from 'react';
import { geoEquirectangular, geoPath } from 'd3-geo';
import { feature } from 'topojson-client';
import type { FeatureCollection } from 'geojson';
import type { GeometryCollection, Topology } from 'topojson-specification';
import type { SatPosition } from '../../api/client';
import { useT } from '../../i18n';
import { footprintPolygon, mmss, nightAlpha, project, sunAltitudeDeg, type LatLon } from '../../lib/satGeo';
import type { AlertPhase, AlertView } from '../../lib/satAlerts';

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
  /** Gli avvisi da disegnare (preavviso, lampo all'AOS, anello fisso, dissolvimento alla LOS). */
  alerts?: readonly AlertView[];
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

const LON_KEY = 'aurora-sat-map-lon';
/** Sotto questo spostamento (px) un gesto e' un clic e non un trascinamento. */
const DRAG_PX = 4;

function loadLon(): number {
  try {
    const v = Number(window.localStorage.getItem(LON_KEY));
    return Number.isFinite(v) ? v : 0;
  } catch {
    return 0;
  }
}

function saveLon(v: number): void {
  try {
    window.localStorage.setItem(LON_KEY, String(v));
  } catch {
    /* memoria del browser non disponibile */
  }
}

/** Rende continue le longitudini di una linea (nessun salto di 360 gradi), cosi' attraversa il bordo senza spezzarsi. */
function unwrap(pts: readonly LatLon[]): LatLon[] {
  const out: LatLon[] = [];
  let prev: number | null = null;
  for (const p of pts) {
    let lon = p[1];
    if (prev !== null) {
      while (lon - prev > 180) lon -= 360;
      while (lon - prev < -180) lon += 360;
    }
    prev = lon;
    out.push([p[0], lon]);
  }
  return out;
}

const PALETTE = { grid: 'rgba(255,255,255,0.16)', label: 'rgba(255,255,255,0.75)', past: 'rgba(255,255,255,0.55)', future: '#ffd84d', foot: '#3ddc84', sat: '#ffffff', sel: '#ffd84d' };

/** La mappa dei satelliti: mondo con giorno e notte, Sole e Luna, la stazione, i satelliti con la loro impronta radio e la traccia del satellite scelto. Si sposta in orizzontale trascinandola e gira senza bordi. */
export function SatMap({ mapUrl, sun, moon, observer, stationLabel, positions, selected, track, trackNowIndex, alerts = [], onSelect }: Props) {
  const t = useT();
  const wrap = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [width, setWidth] = useState(900);
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [countries, setCountries] = useState<FeatureCollection | null>(null);
  const base = useRef<{ w: number; dpr: number; canvas: HTMLCanvasElement } | null>(null);
  const hits = useRef<{ x: number; y: number; norad: number }[]>([]);
  /** Spostamento della mappa in gradi di longitudine (positivo = la mappa va verso destra). */
  const [lonOff, setLonOff] = useState(loadLon);
  const [dragging, setDragging] = useState(false);
  const drag = useRef<{ id: number; x0: number; lon0: number; moved: boolean } | null>(null);
  const shiftPx = useRef(0);

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

  // la notte: un'immagine piccola ingrandita (il bordo e' morbido), ricalcolata solo quando si muove il Sole
  const night = useMemo(() => {
    const nw = 180;
    const nh = 90;
    const nc = document.createElement('canvas');
    nc.width = nw;
    nc.height = nh;
    const nctx = nc.getContext('2d');
    if (!nctx) return null;
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
    return nc;
  }, [sun.lat, sun.lon]);

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

    // la mappa gira come un cilindro: tutto si disegna in copie affiancate spostate di `shift`
    const shift = Math.round((((lonOff / 360) * w) % w + w) % w);
    shiftPx.current = shift;
    /** Esegue `fn` in ogni copia della mappa che tocca lo schermo, per un oggetto che occupa x da xmin a xmax. */
    const eachCopy = (xmin: number, xmax: number, fn: () => void) => {
      const k0 = Math.floor((-shift - xmax) / w) + 1;
      const k1 = Math.ceil((w - shift - xmin) / w) - 1;
      for (let k = k0; k <= k1; k++) {
        ctx.save();
        ctx.translate(shift + k * w, 0);
        fn();
        ctx.restore();
      }
    };

    // fondo: la mappa scelta, la mappa inclusa o (mentre si carica) un gradiente scuro
    if (image) eachCopy(0, w, () => ctx.drawImage(image, 0, 0, w, h));
    else if (countries) {
      if (!base.current || base.current.w !== w || base.current.dpr !== dpr) base.current = { w, dpr, canvas: drawBuiltin(countries, w, h, dpr) };
      const bc = base.current.canvas;
      eachCopy(0, w, () => ctx.drawImage(bc, 0, 0, w, h));
    } else {
      const g = ctx.createLinearGradient(0, 0, 0, h);
      g.addColorStop(0, '#0b1a33');
      g.addColorStop(1, '#0a2440');
      ctx.fillStyle = g;
      ctx.fillRect(0, 0, w, h);
    }

    // la notte
    if (night) {
      ctx.imageSmoothingEnabled = true;
      eachCopy(0, w, () => ctx.drawImage(night, 0, 0, w, h));
    }

    // griglia e scritte sui bordi: le longitudini girano con la mappa, le latitudini restano a sinistra
    ctx.strokeStyle = PALETTE.grid;
    ctx.fillStyle = PALETTE.label;
    ctx.lineWidth = 1;
    ctx.font = '11px system-ui, sans-serif';
    eachCopy(0, w, () => {
      for (let lon = -180; lon <= 150; lon += 30) {
        const [x] = project(0, lon, w, h);
        ctx.beginPath();
        ctx.moveTo(x + 0.5, 0);
        ctx.lineTo(x + 0.5, h);
        ctx.stroke();
        ctx.textAlign = 'center';
        ctx.fillText(`${Math.abs(lon)}${lon < 0 && lon > -180 ? 'W' : lon > 0 ? 'E' : ''}`, x, 12);
      }
    });
    for (let lat = -60; lat <= 60; lat += 30) {
      const [, y] = project(lat, 0, w, h);
      ctx.beginPath();
      ctx.moveTo(0, y + 0.5);
      ctx.lineTo(w, y + 0.5);
      ctx.stroke();
      ctx.textAlign = 'left';
      ctx.fillText(`${Math.abs(lat)}${lat < 0 ? 'S' : lat > 0 ? 'N' : ''}`, 4, y - 3);
    }

    /** Una linea continua (longitudini srotolate) ripetuta in tutte le copie che la mostrano; con `fill` anche riempita. */
    const drawLine = (pts: readonly LatLon[], fill: boolean) => {
      if (pts.length < 2) return;
      const u = unwrap(pts);
      let lo = Infinity;
      let hi = -Infinity;
      for (const p of u) {
        const x = project(p[0], p[1], w, h)[0];
        lo = Math.min(lo, x);
        hi = Math.max(hi, x);
      }
      eachCopy(lo, hi, () => {
        ctx.beginPath();
        u.forEach((p, i) => {
          const [x, y] = project(p[0], p[1], w, h);
          if (i === 0) ctx.moveTo(x, y);
          else ctx.lineTo(x, y);
        });
        if (fill) {
          ctx.closePath();
          ctx.fill();
        }
        ctx.stroke();
      });
    };

    // la traccia a terra: il passato tratteggiato, il futuro pieno
    const past = track.slice(0, Math.max(0, trackNowIndex + 1));
    const future = track.slice(Math.max(0, trackNowIndex));
    ctx.lineWidth = 2;
    ctx.setLineDash([4, 5]);
    ctx.strokeStyle = PALETTE.past;
    drawLine(past, false);
    ctx.setLineDash([]);
    ctx.strokeStyle = PALETTE.future;
    ctx.shadowColor = 'rgba(255,216,77,0.7)';
    ctx.shadowBlur = 6;
    drawLine(future, false);
    ctx.shadowBlur = 0;

    // satelliti: impronta radio, punto e nome
    const placed: { x: number; y: number; norad: number }[] = [];
    for (const p of positions) {
      const isSel = p.norad === selected;
      ctx.lineWidth = isSel ? 2 : 1.2;
      ctx.strokeStyle = PALETTE.foot;
      ctx.fillStyle = isSel ? 'rgba(61,220,132,0.12)' : 'rgba(61,220,132,0.05)';
      drawLine(footprintPolygon(p.lat_deg, p.lon_deg, p.footprint_km), true);
      const [x, y] = project(p.lat_deg, p.lon_deg, w, h);
      placed.push({ x, y, norad: p.norad });
      eachCopy(x - 20, x + 160, () => {
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
      });
    }
    hits.current = placed;

    // la stazione
    if (observer) {
      const [x, y] = project(observer.lat, observer.lon, w, h);
      eachCopy(x - 12, x + 200, () => {
        // la stazione: una stella con un leggero contorno scuro per leggersi sulla mappa
        ctx.beginPath();
        for (let k = 0; k < 10; k++) {
          const r = k % 2 === 0 ? 10 : 4.2;
          const a = -Math.PI / 2 + (k * Math.PI) / 5;
          if (k === 0) ctx.moveTo(x + Math.cos(a) * r, y + Math.sin(a) * r);
          else ctx.lineTo(x + Math.cos(a) * r, y + Math.sin(a) * r);
        }
        ctx.closePath();
        ctx.lineJoin = 'round';
        ctx.lineWidth = 3.2;
        ctx.strokeStyle = 'rgba(0,0,0,0.6)';
        ctx.stroke();
        ctx.fillStyle = '#ff6b6b';
        ctx.fill();
        ctx.fillStyle = '#fff';
        ctx.font = 'bold 12px system-ui, sans-serif';
        ctx.textAlign = 'left';
        ctx.shadowColor = '#000';
        ctx.shadowBlur = 4;
        ctx.fillText(stationLabel, x + 13, y + 16);
        ctx.shadowBlur = 0;
      });
    }

    // il Sole e la Luna
    const [sx, sy] = project(sun.lat, sun.lon, w, h);
    eachCopy(sx - 16, sx + 16, () => {
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
    });
    const [mx, my] = project(moon.lat, moon.lon, w, h);
    eachCopy(mx - 8, mx + 8, () => {
      ctx.fillStyle = '#d9d9d9';
      ctx.shadowColor = '#fff';
      ctx.shadowBlur = 8;
      ctx.beginPath();
      ctx.arc(mx, my, 6, 0, Math.PI * 2);
      ctx.fill();
      ctx.shadowBlur = 0;
    });
  }, [width, image, countries, night, lonOff, sun.lat, sun.lon, moon.lat, moon.lon, observer, stationLabel, positions, selected, track, trackNowIndex]);

  /** Il satellite sotto il punto (x, y) del canvas, tenendo conto dello spostamento e della ripetizione. */
  const pick = (x: number, y: number): number | null => {
    const w = width;
    let best: { norad: number; d: number } | null = null;
    for (const hit of hits.current) {
      const xd = (hit.x + shiftPx.current) % w;
      const d = Math.min(Math.hypot(xd - x, hit.y - y), Math.hypot(xd - w - x, hit.y - y));
      if (d <= 16 && (!best || d < best.d)) best = { norad: hit.norad, d };
    }
    return best ? best.norad : null;
  };

  const endDrag = (e: React.PointerEvent<HTMLCanvasElement>, cancelled: boolean) => {
    const d = drag.current;
    if (!d || d.id !== e.pointerId) return;
    drag.current = null;
    setDragging(false);
    if (d.moved) {
      saveLon(lonOff);
      return;
    }
    if (cancelled) return;
    const r = e.currentTarget.getBoundingClientRect();
    const norad = pick(e.clientX - r.left, e.clientY - r.top);
    if (norad !== null) onSelect(norad);
  };

  return (
    <div className={dragging ? 'sat-map sat-map--drag' : 'sat-map'} ref={wrap}>
      <canvas
        ref={canvas}
        title={t('sat.map.drag')}
        onPointerDown={(e) => {
          if (e.pointerType === 'mouse' && e.button !== 0) return;
          drag.current = { id: e.pointerId, x0: e.clientX, lon0: lonOff, moved: false };
          try {
            e.currentTarget.setPointerCapture(e.pointerId);
          } catch {
            /* il puntatore e' gia' finito */
          }
        }}
        onPointerMove={(e) => {
          const d = drag.current;
          if (!d || d.id !== e.pointerId) return;
          const dx = e.clientX - d.x0;
          if (!d.moved) {
            if (Math.abs(dx) < DRAG_PX) return;
            d.moved = true;
            setDragging(true);
          }
          let v = (d.lon0 + (dx / width) * 360) % 360;
          if (v > 180) v -= 360;
          if (v <= -180) v += 360;
          setLonOff(v);
        }}
        onPointerUp={(e) => endDrag(e, false)}
        onPointerCancel={(e) => endDrag(e, true)}
        onDoubleClick={() => {
          setLonOff(0);
          saveLon(0);
        }}
      />
      {alerts.length > 0 ? <AlertLayer width={width} lonOff={lonOff} observer={observer} positions={positions} alerts={alerts} /> : null}
      <span className="sat-legend">
        <i className="sat-dot sat-dot--sun" /> {t('sat.sun')} <i className="sat-dot sat-dot--moon" /> {t('sat.moon')}
      </span>
    </div>
  );
}

const PHASE_ORDER: Record<AlertPhase, number> = { flash: 0, pre: 1, live: 2, fade: 3 };

interface LayerProps {
  width: number;
  lonOff: number;
  observer: { lat: number; lon: number } | null;
  positions: readonly SatPosition[];
  alerts: readonly AlertView[];
}

/** Anelli e scritte degli avvisi sopra la mappa: un livello SVG che non prende i clic e gira con la mappa (copie affiancate come per il disegno). */
function AlertLayer({ width: w, lonOff, observer, positions, alerts }: LayerProps) {
  const t = useT();
  const h = Math.round(w / 2);
  const shift = Math.round((((lonOff / 360) * w) % w + w) % w);
  /** Le posizioni x di un punto nella mappa spostata, con le copie che toccano lo schermo. */
  const xs = (x: number): number[] => {
    const base = (((x + shift) % w) + w) % w;
    return [base - w, base, base + w].filter((v) => v > -80 && v < w + 80);
  };
  const labelOf = (a: AlertView): string | null => {
    if (a.phase === 'pre') return a.secondsToAos > 0 ? t('sat.alert.pre', { name: a.name, t: mmss(a.secondsToAos) }) : t('sat.alert.soon', { name: a.name });
    if (a.phase === 'fade') return t('sat.alert.los', { name: a.name });
    return null;
  };
  const top = [...alerts].sort((a, b) => PHASE_ORDER[a.phase] - PHASE_ORDER[b.phase])[0];
  const flashing = alerts.filter((a) => a.phase === 'flash');
  const st = observer ? project(observer.lat, observer.lon, w, h) : null;
  return (
    <svg className="sat-alert-layer" width={w} height={h} viewBox={`0 0 ${w} ${h}`} aria-hidden="true">
      {st && top
        ? xs(st[0]).map((x) => (
            <g key={`st-${x}-${top.phase}`} transform={`translate(${x} ${st[1]})`} className={`sat-ring sat-ring--${top.phase}`}>
              <circle className="sat-ring-base" r="12" />
              {top.phase === 'pre' || top.phase === 'flash' ? <circle className="sat-ring-wave" r="12" /> : null}
              {top.phase === 'flash' ? <circle className="sat-ring-wave sat-ring-wave--late" r="12" /> : null}
              {top.phase === 'flash' ? (
                <text className="sat-alert-label sat-alert-label--flash" x="16" y="-14">
                  {t('sat.alert.aos', { name: flashing.map((f) => f.name).join(', ') })}
                </text>
              ) : null}
            </g>
          ))
        : null}
      {alerts.map((a) => {
        const p = positions.find((q) => q.norad === a.norad);
        if (!p) return null;
        const [px, py] = project(p.lat_deg, p.lon_deg, w, h);
        const label = labelOf(a);
        return xs(px).map((x) => (
          <g key={`${a.norad}-${x}-${a.phase}`} transform={`translate(${x} ${py})`} className={`sat-ring sat-ring--${a.phase}`}>
            <circle className="sat-ring-base" r="14" />
            {a.phase === 'pre' || a.phase === 'flash' ? <circle className="sat-ring-wave" r="14" /> : null}
            {label ? (
              <text className="sat-alert-label" x="18" y="22">
                {label}
              </text>
            ) : null}
          </g>
        ));
      })}
    </svg>
  );
}
