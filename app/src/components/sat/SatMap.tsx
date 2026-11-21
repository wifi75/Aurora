import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type RefObject } from 'react';
import { geoEquirectangular, geoPath } from 'd3-geo';
import { feature } from 'topojson-client';
import type { FeatureCollection } from 'geojson';
import type { GeometryCollection, Topology } from 'topojson-specification';
import type { SatPosition } from '../../api/client';
import { useLang, useT } from '../../i18n';
import { footprintPolygon, mmss, nightAlpha, sunAltitudeDeg, type LatLon } from '../../lib/satGeo';
import type { AlertPhase, AlertView } from '../../lib/satAlerts';
import { buildCountryList, countryAt, countryView, type CountryEntry } from '../../lib/satCountries';
import { NIGHT_MAX_LEVEL, PHOTO_LEVELS, photoLevel, terraUrls, type TerraVariant } from '../../lib/satMapChoice';
import { DEFAULT_ZOOM_MODE, FULL_VIEW, ZOOM_MAX, ZOOM_MIN, autoView, clampView, lerpView, panView, projectView, viewsClose, wheelZoomFactor, zoomAt, type MapView, type ZoomMode } from '../../lib/satView';

interface Props {
  /** L'immagine della mappa del mondo (equirettangolare); senza, la mappa inclusa in Aurora (terre e confini di Natural Earth). */
  mapUrl: string | null;
  /** La Terra della NASA (foto di giorno, luci delle citta' di notte); ha la precedenza su `mapUrl`. Finche' le foto caricano resta la mappa vettoriale. */
  terra?: boolean;
  /** Quale foto del giorno con `terra`: con rilievo e fondali (di base) o solo i colori. */
  terraVariant?: TerraVariant;
  /** I confini degli Stati sottili sopra la foto della Terra. */
  borders?: boolean;
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
  /** Lo zoom automatico quando un satellite e' in vista (AOS); alla LOS la mappa torna intera. */
  zoomMode?: ZoomMode;
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

/**
 * La mappa inclusa disegnata una volta per ogni larghezza e risoluzione: mare scuro, terre e confini (proiezione equirettangolare come le mappe in immagine).
 * `dpr` e' il rapporto fra i pixel del disegno e le unita' logiche (pixel dello schermo per la risoluzione, con lo zoom anche di piu'); `line` e' lo spessore dei confini in unita' logiche.
 */
const ISS_NORAD = 25544;

/** Il satellite: un corpo quadrato con due pannelli solari (stessa icona per tutti, colore = quello gia' impostato nel contesto). */
function drawSatIcon(ctx: CanvasRenderingContext2D, x: number, y: number, s: number): void {
  ctx.save();
  ctx.translate(x, y);
  ctx.scale(s, s);
  ctx.fillRect(-3.2, -3.2, 6.4, 6.4);
  ctx.globalAlpha = 0.8;
  ctx.fillRect(-11, -2.6, 6.4, 5.2);
  ctx.fillRect(4.6, -2.6, 6.4, 5.2);
  ctx.restore();
}

/** La Stazione Spaziale: traliccio con quattro coppie di pannelli solari e il modulo centrale. */
function drawIssIcon(ctx: CanvasRenderingContext2D, x: number, y: number, s: number): void {
  ctx.save();
  ctx.translate(x, y);
  ctx.scale(s, s);
  ctx.fillRect(-15, -0.7, 30, 1.4);
  ctx.fillRect(-3, -2.4, 6, 4.8);
  ctx.fillRect(-0.9, -6, 1.8, 12);
  ctx.globalAlpha = 0.85;
  for (const px of [-14.5, -8.8, 4.4, 10.1]) {
    ctx.fillRect(px, -9, 4.4, 7.5);
    ctx.fillRect(px, 1.5, 4.4, 7.5);
  }
  ctx.restore();
}

function drawBuiltin(countries: FeatureCollection, w: number, h: number, dpr: number, line: number, bordersOnly = false): HTMLCanvasElement {
  const c = document.createElement('canvas');
  c.width = Math.round(w * dpr);
  c.height = Math.round(h * dpr);
  const ctx = c.getContext('2d');
  if (!ctx) return c;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  if (!bordersOnly) {
    const sea = ctx.createLinearGradient(0, 0, 0, h);
    sea.addColorStop(0, '#0b1a33');
    sea.addColorStop(1, '#0a2440');
    ctx.fillStyle = sea;
    ctx.fillRect(0, 0, w, h);
  }
  const proj = geoEquirectangular()
    .scale(w / (2 * Math.PI))
    .translate([w / 2, h / 2]);
  const path = geoPath(proj, ctx);
  ctx.beginPath();
  path(countries);
  if (!bordersOnly) {
    ctx.fillStyle = '#25483a';
    ctx.fill();
  }
  ctx.strokeStyle = bordersOnly ? 'rgba(255,255,255,0.4)' : 'rgba(160,210,180,0.35)';
  ctx.lineWidth = line;
  ctx.stroke();
  return c;
}

const LON_KEY = 'aurora-sat-map-lon';
/** Sotto questo spostamento (px) un gesto e' un clic e non un trascinamento. */
const DRAG_PX = 4;
/** Lato massimo (pixel) della mappa inclusa ridisegnata per lo zoom. */
const MAX_BASE_PX = 4096;
/** Costante di tempo (ms) dell'animazione verso una destinazione (zoom automatico, paesi, vista intera). */
const TAU_MS = 140;
/** Costanti di tempo (ms) dello zoom a mano: la rotella risponde quasi subito, i pulsanti un po' piu' morbidi. */
const TAU_WHEEL = 70;
const TAU_BUTTON = 90;
/** Di quanto cambia lo zoom a ogni clic sui pulsanti «+» e «−». */
const BUTTON_ZOOM = 1.6;
/** Dopo questo tempo senza movimenti il disegno si rifa' a qualita' piena («a riposo»). */
const SETTLE_MS = 120;
/** L'interfaccia (pulsanti, avvisi) segue la vista al massimo 8 volte al secondo, piu' a fine movimento. */
const UI_MS = 125;

const reducedMotion = (): boolean => typeof window !== 'undefined' && typeof window.matchMedia === 'function' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
const hasRaf = typeof requestAnimationFrame === 'function';
const schedule = (cb: (now: number) => void): number => (hasRaf ? requestAnimationFrame(cb) : window.setTimeout(() => cb(performance.now()), 16));
const unschedule = (id: number): void => (hasRaf ? cancelAnimationFrame(id) : window.clearTimeout(id));

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

/** La strumentazione dei tempi di disegno e' attiva con `localStorage['aurora.sat.perf']==='1'` (letto una volta all'avvio) o con `window.__satMapPerf = true` (anche a mappa aperta). */
function readPerfFlag(): boolean {
  try {
    return window.localStorage.getItem('aurora.sat.perf') === '1';
  } catch {
    return false;
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

/** Una linea gia' srotolata con i suoi estremi in gradi: si calcola una volta per traccia o impronta, non a ogni fotogramma. */
interface Poly {
  pts: LatLon[];
  lonMin: number;
  lonMax: number;
  latMin: number;
  latMax: number;
}

function polyOf(pts: readonly LatLon[]): Poly {
  const u = unwrap(pts);
  let lonMin = Infinity;
  let lonMax = -Infinity;
  let latMin = Infinity;
  let latMax = -Infinity;
  for (const p of u) {
    lonMin = Math.min(lonMin, p[1]);
    lonMax = Math.max(lonMax, p[1]);
    latMin = Math.min(latMin, p[0]);
    latMax = Math.max(latMax, p[0]);
  }
  return { pts: u, lonMin, lonMax, latMin, latMax };
}

/** Spazio di lavoro per i punti a schermo di una linea (si riusa, niente allocazioni per fotogramma). */
let scratch = new Float64Array(512);

/** Una foto decodificata: bitmap (decodificata fuori dal filo principale) o, dove manca `createImageBitmap`, un elemento immagine. */
type Photo = ImageBitmap | HTMLImageElement;
/** Un'immagine disegnabile con le sue dimensioni in pixel. */
type Img = ImageBitmap | HTMLImageElement | HTMLCanvasElement;

/** Scarica e decodifica una foto senza bloccare la pagina. */
async function fetchPhoto(url: string): Promise<Photo> {
  if (typeof createImageBitmap === 'function') {
    const r = await fetch(url);
    if (!r.ok) throw new Error(`foto non disponibile: ${r.status}`);
    return createImageBitmap(await r.blob());
  }
  const img = new Image();
  img.src = url;
  await img.decode();
  return img;
}

const closePhoto = (p: Photo): void => {
  if ('close' in p) p.close();
};

/** Quante ricostruzioni costose (foto ridotte, notte mascherata, mappa vettoriale) sono state fatte: lo mostra la strumentazione. */
let buildCount = 0;

/** La foto ridotta a `level` pixel di larghezza (la meta' in altezza), a gradini dimezzando: meglio di un solo salto. Le ridotte restano in `cache`. */
function photoAt(cache: Map<number, HTMLCanvasElement>, source: Photo, level: number): Img {
  if (level >= source.width) return source;
  const have = cache.get(level);
  if (have) return have;
  const up = level * 2 >= source.width ? source : photoAt(cache, source, level * 2);
  const c = document.createElement('canvas');
  c.width = level;
  c.height = Math.round(level / 2);
  const x = c.getContext('2d');
  if (!x) return source;
  x.imageSmoothingEnabled = true;
  x.imageSmoothingQuality = 'high';
  x.drawImage(up, 0, 0, c.width, c.height);
  cache.set(level, c);
  buildCount++;
  return c;
}

/**
 * La foto al livello che serve. Durante il movimento (`moving`) non se ne costruiscono di nuove: si prende la piu' piccola gia' pronta che basta
 * o, se non ce n'e', la foto intera (si disegna solo la parte visibile, quindi costa poco); la rifinitura si fa a riposo.
 */
function levelImage(cache: Map<number, HTMLCanvasElement>, source: Photo, want: number, moving: boolean): Img {
  if (want >= source.width) return source;
  const have = cache.get(want);
  if (have) return have;
  if (moving) {
    let best: HTMLCanvasElement | null = null;
    let bestKey = Infinity;
    for (const [k, v] of cache) {
      if (k >= want && k < bestKey) {
        best = v;
        bestKey = k;
      }
    }
    return best ?? source;
  }
  return photoAt(cache, source, want);
}

/** Le foto della Terra ridotte per lo zoom e la notte gia' mascherata dal Sole (si rifa' quando il Sole si e' mosso di un decimo di grado). */
interface PhotoCache {
  photos: { day: Photo; night: Photo } | null;
  day: Map<number, HTMLCanvasElement>;
  nightBase: Map<number, HTMLCanvasElement>;
  sunKey: string;
  nightMasked: Map<number, { key: string; canvas: HTMLCanvasElement }>;
}

/**
 * La foto delle luci mascherata dal Sole al livello `level`. A riposo e' sempre quella del Sole attuale (si rifa' se serve); in movimento si usa
 * qualunque livello gia' pronto (le luci sono sfocate di natura) e si costruisce solo se non ce n'e' nessuno, a bassa risoluzione.
 */
function nightLights(pc: PhotoCache, source: Photo, mask: HTMLCanvasElement, level: number, moving: boolean): HTMLCanvasElement | null {
  const have = pc.nightMasked.get(level);
  if (have && (moving || have.key === pc.sunKey)) return have.canvas;
  let lv = level;
  if (moving) {
    const keys = [...pc.nightMasked.keys()].sort((a, b) => a - b);
    const k = keys.find((x) => x >= level) ?? keys[keys.length - 1];
    const ready = k === undefined ? undefined : pc.nightMasked.get(k);
    if (ready) return ready.canvas;
    lv = Math.min(level, PHOTO_LEVELS[0] ?? level);
  }
  const c2 = document.createElement('canvas');
  c2.width = lv;
  c2.height = Math.round(lv / 2);
  const x = c2.getContext('2d');
  if (!x) return null;
  x.imageSmoothingEnabled = true;
  x.imageSmoothingQuality = 'high';
  x.drawImage(photoAt(pc.nightBase, source, lv), 0, 0, c2.width, c2.height);
  x.globalCompositeOperation = 'destination-in';
  x.drawImage(mask, 0, 0, c2.width, c2.height);
  pc.nightMasked.set(lv, { key: pc.sunKey, canvas: c2 });
  buildCount++;
  return c2;
}

const PALETTE = { grid: 'rgba(255,255,255,0.16)', label: 'rgba(255,255,255,0.75)', past: 'rgba(255,255,255,0.55)', future: '#ffd84d', foot: '#3ddc84', sat: '#ffffff', sel: '#ffd84d' };

/** L'animazione della vista: verso una vista di arrivo (`view`) o solo nello zoom, con il punto sotto il cursore fermo (`zoom`: rotella e pulsanti). */
type Anim = { kind: 'view'; dest: MapView; tau: number } | { kind: 'zoom'; zoom: number; x: number; y: number; tau: number };

/** Le statistiche dei tempi di `draw()` (ultimi 120 fotogrammi), solo con la strumentazione accesa. */
interface PerfState {
  ms: Float64Array;
  n: number;
  frames: number;
  since: number;
  lastText: number;
}

/**
 * Posiziona gli anelli degli avvisi (gruppi SVG con `data-lat`, `data-lon`, `data-k`) sulla vista corrente senza passare da React:
 * si chiama a ogni disegno, quindi gli anelli restano agganciati ai satelliti mentre la mappa si muove.
 */
function placeAlerts(svg: SVGSVGElement | null, w: number, h: number, view: MapView): void {
  if (!svg) return;
  const worldW = w * view.zoom;
  svg.querySelectorAll<SVGGElement>('g[data-lat]').forEach((g) => {
    const [px, py] = projectView(Number(g.dataset.lat), Number(g.dataset.lon), w, h, view);
    const x = px + Number(g.dataset.k) * worldW;
    g.setAttribute('transform', `translate(${x.toFixed(1)} ${py.toFixed(1)})`);
    const vis = x > -80 && x < w + 80 ? 'visible' : 'hidden';
    if (g.getAttribute('visibility') !== vis) g.setAttribute('visibility', vis);
  });
}

/**
 * La mappa dei satelliti: mondo con giorno e notte, Sole e Luna, la stazione, i satelliti con la loro impronta radio e la traccia del satellite scelto. Si sposta trascinandola, si ingrandisce con la rotella, il pizzico o i pulsanti e gira senza bordi; con un satellite in vista si inquadra da sola.
 *
 * Il disegno non passa dallo stato di React: la vista corrente sta in `viewRef`, ogni cambiamento chiede un disegno e un solo `requestAnimationFrame` per fotogramma
 * (piu' eventi nello stesso fotogramma si fondono) ridisegna il canvas. In movimento la qualita' e' ridotta, a riposo (120 ms dopo l'ultimo movimento) si rifinisce.
 * Lo stato di React con la vista serve solo all'interfaccia (pulsanti attivi) e si aggiorna al massimo 8 volte al secondo e a fine movimento.
 */
export function SatMap({ mapUrl, terra = false, terraVariant = 'relief', borders = false, sun, moon, observer, stationLabel, positions, selected, track, trackNowIndex, alerts = [], zoomMode = DEFAULT_ZOOM_MODE, onSelect }: Props) {
  const t = useT();
  const lang = useLang((st) => st.lang);
  const wrap = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const layer = useRef<SVGSVGElement>(null);
  const perfEl = useRef<HTMLSpanElement>(null);
  const [width, setWidth] = useState(900);
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [countries, setCountries] = useState<FeatureCollection | null>(null);
  const [photos, setPhotos] = useState<{ day: Photo; night: Photo } | null>(null);
  type VectorBase = { w: number; dpr: number; bucket: number; res: number; canvas: HTMLCanvasElement } | null;
  const base = useRef<VectorBase>(null);
  const bordersBase = useRef<VectorBase>(null);
  const photoCache = useRef<PhotoCache>({ photos: null, day: new Map(), nightBase: new Map(), sunKey: '', nightMasked: new Map() });
  const hits = useRef<{ x: number; y: number; norad: number }[]>([]);
  /** Larghezza in pixel del mondo intero all'ultimo disegno (la mappa si ripete a questa distanza). */
  const hitW = useRef(900);
  /** Il centro della mappa a vista intera, deciso a mano e salvato (`aurora-sat-map-lon` e' lo spostamento, cioe' il suo opposto). La vista automatica non lo tocca. */
  const restLon = useRef(-loadLon());
  const [view, setView] = useState<MapView>(() => clampView({ zoom: 1, centerLat: 0, centerLon: restLon.current }));
  /** La vista vera, aggiornata a ogni fotogramma; `view` (stato) la segue a rate ridotto. */
  const viewRef = useRef(view);
  const uiView = useRef(view);
  const sizeRef = useRef({ w: 900, h: 450 });
  const [dragging, setDragging] = useState(false);
  const drag = useRef<{ id: number; x0: number; y0: number; view0: MapView; moved: boolean } | null>(null);
  const pointers = useRef(new Map<number, { x: number; y: number }>());
  const pinch = useRef<{ d0: number; view0: MapView } | null>(null);
  // animazione della vista, disegno coalescente, rifinitura a riposo, aggiornamento dell'interfaccia
  const anim = useRef<Anim | null>(null);
  const animRaf = useRef<number | null>(null);
  const lastTick = useRef(0);
  const drawRaf = useRef<number | null>(null);
  const dirty = useRef(false);
  const settleTimer = useRef<number | null>(null);
  const moveStamp = useRef(-1e9);
  const uiTimer = useRef<number | null>(null);
  const lastUi = useRef(0);
  const paintRef = useRef<((moving: boolean) => void) | null>(null);
  // zoom automatico: se ha inquadrato il passaggio, se l'utente l'ha sospeso con un gesto, se un satellite e' in vista
  const engaged = useRef(false);
  const suspended = useRef(false);
  const inPass = useRef(false);
  const [perfAtStart] = useState(readPerfFlag);
  const perf = useRef<PerfState | null>(null);
  const footCache = useRef(new WeakMap<SatPosition, Poly>());

  sizeRef.current = { w: width, h: Math.round(width / 2) };

  /** Tutto il controllo del movimento: vive per tutta la vita del componente e tocca solo i riferimenti, mai lo stato di React per fotogramma. */
  const ctl = useMemo(() => {
    const isMoving = () => performance.now() - moveStamp.current < SETTLE_MS;
    /** Porta lo stato di React alla vista corrente. */
    const flushUi = () => {
      if (uiTimer.current !== null) window.clearTimeout(uiTimer.current);
      uiTimer.current = null;
      lastUi.current = performance.now();
      const v = viewRef.current;
      const u = uiView.current;
      if (v.zoom === u.zoom && v.centerLat === u.centerLat && v.centerLon === u.centerLon) return;
      uiView.current = v;
      setView(v);
    };
    /** Al massimo ogni `UI_MS`: subito se e' passato abbastanza (il primo evento di un gesto), altrimenti in coda. */
    const scheduleUi = () => {
      const dt = performance.now() - lastUi.current;
      if (dt >= UI_MS) flushUi();
      else if (uiTimer.current === null) uiTimer.current = window.setTimeout(flushUi, UI_MS - dt);
    };
    const paintNow = () => {
      dirty.current = false;
      paintRef.current?.(isMoving());
    };
    /** Un movimento appena avvenuto: a riposo (dopo `SETTLE_MS`) si ridisegna a qualita' piena e si allinea l'interfaccia. */
    const markMove = () => {
      moveStamp.current = performance.now();
      if (settleTimer.current !== null) window.clearTimeout(settleTimer.current);
      settleTimer.current = window.setTimeout(() => {
        settleTimer.current = null;
        flushUi();
        dirty.current = false;
        paintRef.current?.(false);
      }, SETTLE_MS);
    };
    const frame = () => {
      drawRaf.current = null;
      if (!dirty.current) return;
      markMove();
      paintNow();
    };
    /** Chiede un disegno: piu' richieste nello stesso fotogramma ne fanno uno solo. */
    const requestDraw = () => {
      dirty.current = true;
      if (drawRaf.current === null) drawRaf.current = schedule(frame);
    };
    /** Imposta la vista subito (trascinamento, pizzico, ripristino): nessuna animazione. */
    const commit = (v: MapView) => {
      viewRef.current = v;
      requestDraw();
      scheduleUi();
    };
    const stopAnim = () => {
      if (animRaf.current !== null) unschedule(animRaf.current);
      animRaf.current = null;
      anim.current = null;
    };
    const step = (now: number) => {
      animRaf.current = null;
      const a = anim.current;
      if (!a) return;
      const dt = Math.max(0, Math.min(100, now - lastTick.current));
      lastTick.current = now;
      const f = 1 - Math.exp(-dt / a.tau);
      const cur = viewRef.current;
      let next: MapView;
      let done: boolean;
      if (a.kind === 'zoom') {
        // zoom logaritmico con il punto sotto il cursore sempre fermo (si riapplica a ogni fotogramma dalla vista corrente)
        const { w, h } = sizeRef.current;
        const lz = Math.log(cur.zoom);
        const lg = Math.log(a.zoom);
        done = Math.abs(lg - lz) < 0.003;
        const z = done ? a.zoom : Math.exp(lz + (lg - lz) * f);
        next = zoomAt(cur, z / cur.zoom, a.x, a.y, w, h);
      } else {
        done = viewsClose(cur, a.dest);
        next = done ? a.dest : clampView(lerpView(cur, a.dest, f));
      }
      viewRef.current = next;
      markMove();
      paintNow();
      scheduleUi();
      if (done) {
        anim.current = null;
        flushUi();
        return;
      }
      animRaf.current = schedule(step);
    };
    const ensureAnim = () => {
      if (animRaf.current === null) {
        lastTick.current = performance.now();
        animRaf.current = schedule(step);
      }
    };
    /** Porta la vista a `dest` con un movimento morbido (subito, con «riduci animazioni»). */
    const animateTo = (dest: MapView, tau = TAU_MS) => {
      const d = clampView(dest);
      if (reducedMotion()) {
        stopAnim();
        commit(d);
        return;
      }
      anim.current = { kind: 'view', dest: d, tau };
      ensureAnim();
    };
    /** Porta lo zoom a `zoom` tenendo fermo il punto del mondo sotto (x, y) del canvas. */
    const animateZoom = (zoom: number, x: number, y: number, tau: number) => {
      const z = Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, zoom));
      if (reducedMotion()) {
        stopAnim();
        const { w, h } = sizeRef.current;
        const cur = viewRef.current;
        commit(zoomAt(cur, z / cur.zoom, x, y, w, h));
        return;
      }
      anim.current = { kind: 'zoom', zoom: z, x, y, tau };
      ensureAnim();
    };
    /** Lo zoom a cui sta andando la mappa (o quello attuale): i colpi di rotella e i clic si sommano su questo. */
    const goalZoom = (): number => {
      const a = anim.current;
      return a ? (a.kind === 'zoom' ? a.zoom : a.dest.zoom) : viewRef.current.zoom;
    };
    /** Un gesto dell'utente: se c'e' un satellite in vista sospende lo zoom automatico fino al prossimo AOS. */
    const noteManual = () => {
      engaged.current = false;
      if (inPass.current) suspended.current = true;
    };
    /** Un gesto che prende il comando: ferma l'animazione. */
    const manual = () => {
      stopAnim();
      noteManual();
    };
    const dispose = () => {
      stopAnim();
      if (drawRaf.current !== null) unschedule(drawRaf.current);
      drawRaf.current = null;
      if (settleTimer.current !== null) window.clearTimeout(settleTimer.current);
      settleTimer.current = null;
      if (uiTimer.current !== null) window.clearTimeout(uiTimer.current);
      uiTimer.current = null;
    };
    return { isMoving, flushUi, paintNow, commit, stopAnim, animateTo, animateZoom, goalZoom, animating: () => anim.current !== null, noteManual, manual, dispose };
    // tutto passa da riferimenti: il controllo si crea una volta sola
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const restView = (): MapView => clampView({ zoom: 1, centerLat: 0, centerLon: restLon.current });
  const buttonZoom = (factor: number) => {
    const { w, h } = sizeRef.current;
    ctl.noteManual();
    ctl.animateZoom(ctl.goalZoom() * factor, w / 2, h / 2, TAU_BUTTON);
  };

  useEffect(() => () => ctl.dispose(), [ctl]);

  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setWidth(Math.max(320, Math.floor(el.clientWidth))));
    ro.observe(el);
    setWidth(Math.max(320, Math.floor(el.clientWidth)));
    return () => ro.disconnect();
  }, []);

  // le terre vettoriali servono sempre: mappa semplice, ripiego mentre caricano le foto, confini, elenco dei paesi
  useEffect(() => {
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
  }, []);

  // la mappa scelta dall'utente (un'immagine)
  useEffect(() => {
    if (!mapUrl || terra) {
      setImage(null);
      return;
    }
    const img = new Image();
    img.onload = () => setImage(img);
    img.onerror = () => setImage(null);
    img.src = mapUrl;
  }, [mapUrl, terra]);

  // la Terra della NASA: due foto grandi, decodificate una volta fuori dal filo principale e rilasciate quando non servono
  useEffect(() => {
    if (!terra) {
      setPhotos(null);
      return;
    }
    let alive = true;
    const urls = terraUrls(terraVariant);
    void Promise.all([fetchPhoto(urls.day), fetchPhoto(urls.night)]).then(
      ([day, night]) => {
        if (!alive) {
          closePhoto(day);
          closePhoto(night);
          return;
        }
        setPhotos({ day, night });
      },
      () => undefined,
    );
    return () => {
      alive = false;
      setPhotos((p) => {
        if (p) {
          closePhoto(p.day);
          closePhoto(p.night);
        }
        return null;
      });
    };
  }, [terra, terraVariant]);
  const photoReady = terra && photos !== null;

  // zoom automatico: il satellite in vista (fase lampo o fissa degli avvisi), quello scelto se ce n'e' piu' d'uno, altrimenti il primo
  const liveAlerts = useMemo(() => alerts.filter((a) => a.phase === 'flash' || a.phase === 'live'), [alerts]);
  const anyLive = liveAlerts.length > 0;
  const desired = useMemo<MapView | null>(() => {
    if (zoomMode === 'off') return null;
    const a = liveAlerts.find((x) => x.norad === selected) ?? liveAlerts[0];
    const p = a ? positions.find((q) => q.norad === a.norad) : undefined;
    return p ? autoView(zoomMode, { lat: p.lat_deg, lon: p.lon_deg, footprintKm: p.footprint_km }, observer) : null;
  }, [zoomMode, liveAlerts, positions, selected, observer]);
  const desiredKey = desired ? `${desired.zoom.toFixed(3)}|${desired.centerLat.toFixed(2)}|${desired.centerLon.toFixed(2)}` : '';
  useEffect(() => {
    inPass.current = anyLive;
    if (desired) {
      if (suspended.current) return;
      engaged.current = true;
      ctl.animateTo(desired);
      return;
    }
    // nessun satellite in vista (LOS) o zoom spento: se la vista l'ha decisa lo zoom automatico, si torna intera
    if (engaged.current) {
      engaged.current = false;
      ctl.animateTo(restView());
    }
    if (!anyLive) suspended.current = false;
    // la vista si rinnova solo quando cambia davvero (la stazione e le posizioni arrivano come nuovi oggetti a ogni aggiornamento)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [desiredKey, anyLive]);

  // la rotella ingrandisce intorno al cursore; l'ascoltatore e' diretto perche' deve poter fermare lo scorrimento della pagina.
  // Ogni colpo somma il suo fattore allo zoom di arrivo e la mappa lo raggiunge in pochi fotogrammi (punto sotto il cursore fermo).
  useEffect(() => {
    const c = canvas.current;
    if (!c) return;
    const onWheel = (e: WheelEvent) => {
      const factor = wheelZoomFactor(e.deltaY, e.deltaMode);
      if (factor === 1) return;
      // a vista intera la rotella verso il basso lascia scorrere la pagina
      if (factor < 1 && viewRef.current.zoom <= ZOOM_MIN + 1e-6 && !ctl.animating()) return;
      e.preventDefault();
      const r = c.getBoundingClientRect();
      const { w } = sizeRef.current;
      const sc = r.width > 0 ? w / r.width : 1;
      ctl.noteManual();
      ctl.animateZoom(ctl.goalZoom() * factor, (e.clientX - r.left) * sc, (e.clientY - r.top) * sc, TAU_WHEEL);
    };
    c.addEventListener('wheel', onWheel, { passive: false });
    return () => c.removeEventListener('wheel', onWheel);
  }, [ctl]);

  // la notte: un'immagine piccola ingrandita (il bordo e' morbido), ricalcolata solo quando si muove il Sole;
  // con la Terra della NASA serve solo il canale alfa, come maschera della foto delle luci
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
        const a = nightAlpha(sunAltitudeDeg(lat, lon, sun.lat, sun.lon), photoReady ? 0.68 : 0.74);
        const o = (y * nw + x) * 4;
        img.data[o] = 4;
        img.data[o + 1] = 10;
        img.data[o + 2] = 38;
        img.data[o + 3] = Math.round(a * 255);
      }
    }
    nctx.putImageData(img, 0, 0);
    return nc;
  }, [sun.lat, sun.lon, photoReady]);

  // la traccia srotolata una volta sola (cambia solo con una traccia nuova)
  const trackPoly = useMemo(() => ({ past: polyOf(track.slice(0, Math.max(0, trackNowIndex + 1))), future: polyOf(track.slice(Math.max(0, trackNowIndex))) }), [track, trackNowIndex]);

  // l'elenco dei paesi nella lingua dell'interfaccia e il paese della stazione
  const countryList = useMemo(() => (countries ? buildCountryList(countries, lang) : []), [countries, lang]);
  const myCountry = useMemo(() => {
    if (!countries || !observer) return null;
    const i = countryAt(countries, observer.lat, observer.lon);
    return i === null ? null : (countryList.find((c) => c.index === i) ?? null);
  }, [countries, countryList, observer]);
  const countryListRef = useRef(countryList);
  countryListRef.current = countryList;
  const goCountry = useCallback(
    (index: number) => {
      const c = countryListRef.current.find((x) => x.index === index);
      if (!c) return;
      ctl.manual();
      ctl.animateTo(countryView(c.box));
    },
    [ctl],
  );

  /** Il disegno completo con la vista corrente (`viewRef`); `moving` = qualita' ridotta e nessuna ricostruzione costosa. Si richiama da `requestAnimationFrame` o a ogni cambio dei dati. */
  const paint = (moving: boolean) => {
    const c = canvas.current;
    const ctx = c?.getContext('2d');
    if (!c || !ctx) return;
    const perfOn = (window as unknown as { __satMapPerf?: boolean }).__satMapPerf === true || perfAtStart;
    const t0 = perfOn ? performance.now() : 0;
    const view = viewRef.current;
    const dpr = Math.min(2, window.devicePixelRatio || 1);
    const w = width;
    const h = Math.round(width / 2);
    // la dimensione del canvas si tocca solo se cambia (assegnarla cancella e rialloca il disegno)
    const pxW = Math.round(w * dpr);
    const pxH = Math.round(h * dpr);
    if (c.width !== pxW) c.width = pxW;
    if (c.height !== pxH) c.height = pxH;
    const cssH = `${h}px`;
    if (c.style.height !== cssH) c.style.height = cssH;
    ctx.save();
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    // la vista: con zoom z il mondo e' largo W = w*z e alto H = h*z; (left, top) e' l'angolo in alto a sinistra della copia centrale
    const z = view.zoom;
    const W = w * z;
    const H = h * z;
    const kx = W / 360;
    const ky = H / 180;
    const left = Math.round(w / 2 - ((view.centerLon + 180) / 360) * W);
    const top = Math.round(h / 2 - ((90 - view.centerLat) / 180) * H);
    hitW.current = W;
    /** La proiezione con zoom e spostamento: i simboli si disegnano qui, sempre della stessa grandezza in pixel. */
    const pr = (lat: number, lon: number) => projectView(lat, lon, w, h, view);
    /** Il bagliore (ombra sfocata) costa molto: in movimento si salta. */
    const glow = (n: number) => (moving ? 0 : n);
    /** Esegue `fn` in ogni copia del mondo che tocca lo schermo, per un oggetto che occupa x da xmin a xmax. */
    const eachCopy = (xmin: number, xmax: number, fn: () => void) => {
      const k0 = Math.floor(-xmax / W) + 1;
      const k1 = Math.ceil((w - xmin) / W) - 1;
      for (let k = k0; k <= k1; k++) {
        ctx.save();
        ctx.translate(k * W, 0);
        fn();
        ctx.restore();
      }
    };
    /**
     * Disegna un'immagine che copre il mondo (una per copia): solo la parte visibile, con rettangolo sorgente e destinazione, senza scalare il canvas.
     * Il rettangolo sorgente e' allargato al pixel intero cosi' i bordi cadono fuori dallo schermo.
     */
    const drawWorld = (src: Img) => {
      const sw = src.width;
      const sh = src.height;
      if (sw <= 0 || sh <= 0) return;
      const fx = sw / w;
      const fy = sh / h;
      const ly0 = Math.max(0, -top / z);
      const ly1 = Math.min(h, (h - top) / z);
      const sy0 = Math.max(0, Math.floor(ly0 * fy));
      const sy1 = Math.min(sh, Math.ceil(ly1 * fy));
      if (sy1 <= sy0) return;
      const k0 = Math.floor(-(left + W) / W) + 1;
      const k1 = Math.ceil((w - left) / W) - 1;
      for (let k = k0; k <= k1; k++) {
        const tx = left + k * W;
        const lx0 = Math.max(0, -tx / z);
        const lx1 = Math.min(w, (w - tx) / z);
        const sx0 = Math.max(0, Math.floor(lx0 * fx));
        const sx1 = Math.min(sw, Math.ceil(lx1 * fx));
        if (sx1 <= sx0) continue;
        ctx.drawImage(src, sx0, sy0, sx1 - sx0, sy1 - sy0, tx + (sx0 / fx) * z, top + (sy0 / fy) * z, ((sx1 - sx0) / fx) * z, ((sy1 - sy0) / fy) * z);
      }
    };

    // sotto a tutto il mare scuro (copre le fessure fra le copie e il tempo del caricamento)
    const g = ctx.createLinearGradient(0, 0, 0, h);
    g.addColorStop(0, '#0b1a33');
    g.addColorStop(1, '#0a2440');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, w, h);

    /** Il livello vettoriale (terre e confini, o solo i confini) alla risoluzione che serve allo zoom, fino a 4096 px. In movimento si tiene quello che c'e' (la rifinitura a riposo lo rifa'). */
    const vectorLayer = (cache: { current: VectorBase }, bordersOnly: boolean): HTMLCanvasElement | null => {
      if (!countries) return null;
      const b = cache.current;
      if (moving && b && b.w === w && b.dpr === dpr) return b.canvas;
      const bucket = z <= 1 ? 1 : z <= 2 ? 2 : z <= 4 ? 4 : 8;
      let res = bucket;
      while (res > 1 && w * dpr * res > MAX_BASE_PX) res /= 2;
      if (!b || b.w !== w || b.dpr !== dpr || b.bucket !== bucket || b.res !== res) {
        cache.current = { w, dpr, bucket, res, canvas: drawBuiltin(countries, w, h, dpr * res, (bordersOnly ? 0.5 : 0.65) / bucket, bordersOnly) };
        buildCount++;
      }
      return cache.current?.canvas ?? null;
    };

    // le foto ridotte restano in memoria finche' sono le stesse; il Sole mosso di un decimo di grado rifa' la notte (a riposo)
    const pc = photoCache.current;
    if (pc.photos !== (photoReady ? photos : null)) {
      pc.photos = photoReady ? photos : null;
      pc.day.clear();
      pc.nightBase.clear();
      pc.nightMasked.clear();
      // le copie ridotte si preparano subito (una per volta, fra un fotogramma e l'altro): se si costruissero durante lo zoom con la rotella,
      // il primo movimento disegnerebbe dalla foto intera in qualita' bassa e la mappa apparirebbe sgranata
      const cur = pc.photos;
      if (cur) {
        PHOTO_LEVELS.forEach((lv, i) => {
          window.setTimeout(() => {
            if (photoCache.current.photos !== cur) return;
            photoAt(pc.day, cur.day, lv);
            if (lv <= NIGHT_MAX_LEVEL) photoAt(pc.nightBase, cur.night, lv);
          }, 60 * (i + 1));
        });
      }
    }
    pc.sunKey = `${Math.round(sun.lat * 10)},${Math.round(sun.lon * 10)}`;
    const wanted = photoLevel(w * dpr * z);

    // fondo: la Terra della NASA, la mappa dell'utente o la mappa semplice
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = moving ? 'low' : 'high';
    let veil = true;
    if (photoReady && photos) {
      drawWorld(levelImage(pc.day, photos.day, wanted, moving));
      veil = false;
    } else if (image) drawWorld(image);
    else {
      const bc = vectorLayer(base, false);
      if (bc) drawWorld(bc);
    }

    // la notte: con la Terra la foto delle luci mascherata dal Sole (bordo morbido), altrimenti un velo blu scuro
    if (!veil && photos && night) {
      // la Terra di giorno resta visibile, scurita da un velo (coste, rilievi e nuvole nitidi); le luci delle citta' ci si sommano sopra
      drawWorld(night);
      const lt = nightLights(pc, photos.night, night, Math.min(wanted, NIGHT_MAX_LEVEL), moving);
      if (lt) {
        ctx.globalCompositeOperation = 'screen';
        drawWorld(lt);
        ctx.globalCompositeOperation = 'source-over';
      }
    } else if (night) drawWorld(night);

    // i confini sottili sopra la foto (opzionali)
    if (borders && photoReady) {
      const bl = vectorLayer(bordersBase, true);
      if (bl) drawWorld(bl);
    }

    // griglia e scritte sui bordi (spessori e caratteri non crescono con lo zoom): le longitudini girano con la mappa, le latitudini restano a sinistra
    const gridStep = z >= 6 ? 5 : z >= 3 ? 10 : z >= 1.8 ? 15 : 30;
    ctx.strokeStyle = PALETTE.grid;
    ctx.fillStyle = PALETTE.label;
    ctx.lineWidth = 1;
    ctx.font = '11px system-ui, sans-serif';
    ctx.textAlign = 'center';
    for (let lon = -180; lon < 180; lon += gridStep) {
      const [x] = pr(0, lon);
      if (x < -1 || x > w + 1) continue;
      ctx.beginPath();
      ctx.moveTo(x + 0.5, 0);
      ctx.lineTo(x + 0.5, h);
      ctx.stroke();
      ctx.fillText(`${Math.abs(lon)}${lon < 0 && lon > -180 ? 'W' : lon > 0 ? 'E' : ''}`, x, 12);
    }
    ctx.textAlign = 'left';
    for (let lat = -90 + gridStep; lat < 90; lat += gridStep) {
      const [, y] = pr(lat, 0);
      if (y < -1 || y > h + 1) continue;
      ctx.beginPath();
      ctx.moveTo(0, y + 0.5);
      ctx.lineTo(w, y + 0.5);
      ctx.stroke();
      ctx.fillText(`${Math.abs(lat)}${lat < 0 ? 'S' : lat > 0 ? 'N' : ''}`, 4, y - 3);
    }

    /** Una linea continua (longitudini gia' srotolate) ripetuta in tutte le copie che la mostrano; con `fill` anche riempita. I punti a schermo si calcolano una volta e valgono per ogni copia. */
    const drawPoly = (poly: Poly, fill: boolean) => {
      const n = poly.pts.length;
      if (n < 2) return;
      const yTop = h / 2 - (poly.latMax - view.centerLat) * ky;
      const yBot = h / 2 - (poly.latMin - view.centerLat) * ky;
      if (yBot < -4 || yTop > h + 4) return;
      if (scratch.length < n * 2) scratch = new Float64Array(n * 2);
      const pts = scratch;
      for (let i = 0; i < n; i++) {
        const p = poly.pts[i];
        if (!p) continue;
        pts[i * 2] = w / 2 + (p[1] - view.centerLon) * kx;
        pts[i * 2 + 1] = h / 2 - (p[0] - view.centerLat) * ky;
      }
      eachCopy(w / 2 + (poly.lonMin - view.centerLon) * kx, w / 2 + (poly.lonMax - view.centerLon) * kx, () => {
        ctx.beginPath();
        ctx.moveTo(pts[0] ?? 0, pts[1] ?? 0);
        for (let i = 1; i < n; i++) ctx.lineTo(pts[i * 2] ?? 0, pts[i * 2 + 1] ?? 0);
        if (fill) {
          ctx.closePath();
          ctx.fill();
        }
        ctx.stroke();
      });
    };

    // la traccia a terra: il passato tratteggiato, il futuro pieno
    ctx.lineWidth = 2;
    ctx.setLineDash([4, 5]);
    ctx.strokeStyle = PALETTE.past;
    drawPoly(trackPoly.past, false);
    ctx.setLineDash([]);
    ctx.strokeStyle = PALETTE.future;
    ctx.shadowColor = 'rgba(255,216,77,0.7)';
    ctx.shadowBlur = glow(6);
    drawPoly(trackPoly.future, false);
    ctx.shadowBlur = 0;

    // satelliti: impronta radio, punto e nome
    const placed: { x: number; y: number; norad: number }[] = [];
    for (const p of positions) {
      const isSel = p.norad === selected;
      ctx.lineWidth = isSel ? 2 : 1.2;
      ctx.strokeStyle = PALETTE.foot;
      ctx.fillStyle = isSel ? 'rgba(61,220,132,0.12)' : 'rgba(61,220,132,0.05)';
      let foot = footCache.current.get(p);
      if (!foot) {
        foot = polyOf(footprintPolygon(p.lat_deg, p.lon_deg, p.footprint_km));
        footCache.current.set(p, foot);
      }
      drawPoly(foot, true);
      const [x, y] = pr(p.lat_deg, p.lon_deg);
      placed.push({ x, y, norad: p.norad });
      if (y < -24 || y > h + 24) continue;
      eachCopy(x - 20, x + 160, () => {
        ctx.fillStyle = isSel ? PALETTE.sel : PALETTE.sat;
        ctx.shadowColor = isSel ? PALETTE.sel : '#9ad';
        ctx.shadowBlur = glow(isSel ? 12 : 6);
        if (p.norad === ISS_NORAD) drawIssIcon(ctx, x, y, isSel ? 0.7 : 0.62);
        else drawSatIcon(ctx, x, y, isSel ? 0.58 : 0.5);
        ctx.shadowBlur = 0;
        if (isSel) {
          ctx.strokeStyle = PALETTE.sel;
          ctx.lineWidth = 1.5;
          ctx.beginPath();
          ctx.arc(x, y, p.norad === ISS_NORAD ? 14 : 10, 0, Math.PI * 2);
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
      const [x, y] = pr(observer.lat, observer.lon);
      eachCopy(x - 12, x + 200, () => {
        // la stazione: una stella con un leggero contorno scuro per leggersi sulla mappa
        ctx.beginPath();
        for (let k = 0; k < 10; k++) {
          const r = k % 2 === 0 ? 7 : 3;
          const a = -Math.PI / 2 + (k * Math.PI) / 5;
          if (k === 0) ctx.moveTo(x + Math.cos(a) * r, y + Math.sin(a) * r);
          else ctx.lineTo(x + Math.cos(a) * r, y + Math.sin(a) * r);
        }
        ctx.closePath();
        ctx.lineJoin = 'round';
        ctx.lineWidth = 2.4;
        ctx.strokeStyle = 'rgba(0,0,0,0.6)';
        ctx.stroke();
        ctx.fillStyle = '#ff6b6b';
        ctx.fill();
        ctx.fillStyle = '#fff';
        ctx.font = 'bold 12px system-ui, sans-serif';
        ctx.textAlign = 'left';
        ctx.shadowColor = '#000';
        ctx.shadowBlur = 4;
        ctx.fillText(stationLabel, x + 10, y + 14);
        ctx.shadowBlur = 0;
      });
    }

    // il Sole e la Luna
    const [sx, sy] = pr(sun.lat, sun.lon);
    eachCopy(sx - 16, sx + 16, () => {
      ctx.fillStyle = '#ffd84d';
      ctx.shadowColor = '#ffb300';
      ctx.shadowBlur = glow(16);
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
    const [mx, my] = pr(moon.lat, moon.lon);
    eachCopy(mx - 8, mx + 8, () => {
      ctx.fillStyle = '#d9d9d9';
      ctx.shadowColor = '#fff';
      ctx.shadowBlur = glow(8);
      ctx.beginPath();
      ctx.arc(mx, my, 6, 0, Math.PI * 2);
      ctx.fill();
      ctx.shadowBlur = 0;
    });
    ctx.restore();

    // gli anelli degli avvisi seguono la stessa vista, senza render di React
    placeAlerts(layer.current, w, h, view);

    // strumentazione (spenta di base: qui non costa nulla)
    const el = perfEl.current;
    if (!perfOn) {
      if (el && !el.hidden) el.hidden = true;
      return;
    }
    const now = performance.now();
    const ps = (perf.current ??= { ms: new Float64Array(120), n: 0, frames: 0, since: now, lastText: 0 });
    ps.ms[ps.n % 120] = now - t0;
    ps.n++;
    ps.frames++;
    if (el && now - ps.lastText >= 500) {
      const cnt = Math.min(ps.n, 120);
      let sum = 0;
      let max = 0;
      for (let i = 0; i < cnt; i++) {
        const v = ps.ms[i] ?? 0;
        sum += v;
        if (v > max) max = v;
      }
      const fps = (ps.frames * 1000) / Math.max(1, now - ps.since);
      el.textContent = `${(sum / cnt).toFixed(1)} ms (max ${max.toFixed(0)}) · ${fps.toFixed(0)} fps · ${moving ? 'mov' : 'fermo'} · rifatti ${buildCount}`;
      el.hidden = false;
      ps.lastText = now;
      ps.frames = 0;
      ps.since = now;
    }
  };

  // il disegno vede sempre i dati dell'ultimo render
  useLayoutEffect(() => {
    paintRef.current = paint;
  });
  // gli anelli degli avvisi, appena rinnovati da React, si posizionano subito sulla vista corrente
  useLayoutEffect(() => {
    placeAlerts(layer.current, width, Math.round(width / 2), viewRef.current);
  });

  // cambiano i dati (non la vista): si ridisegna subito (con il canvas ridimensionato il disegno sarebbe cancellato)
  useEffect(() => {
    ctl.paintNow();
  }, [ctl, width, image, countries, photos, photoReady, borders, night, sun.lat, sun.lon, moon.lat, moon.lon, observer, stationLabel, positions, selected, trackPoly]);

  /** Il satellite sotto il punto (x, y) del canvas, tenendo conto della ripetizione del mondo. */
  const pick = (x: number, y: number): number | null => {
    const W = hitW.current;
    let best: { norad: number; d: number } | null = null;
    for (const hit of hits.current) {
      for (const k of [-1, 0, 1]) {
        const d = Math.hypot(hit.x + k * W - x, hit.y - y);
        if (d <= 16 && (!best || d < best.d)) best = { norad: hit.norad, d };
      }
    }
    return best ? best.norad : null;
  };

  const endDrag = (e: React.PointerEvent<HTMLCanvasElement>, cancelled: boolean) => {
    pointers.current.delete(e.pointerId);
    if (pinch.current && pointers.current.size < 2) pinch.current = null;
    const d = drag.current;
    if (!d || d.id !== e.pointerId) return;
    drag.current = null;
    setDragging(false);
    if (d.moved) {
      // lo spostamento a mano della mappa intera si ricorda (solo a fine gesto); da ingrandita no (e la vista automatica non si salva mai)
      const v = viewRef.current;
      if (v.zoom <= ZOOM_MIN + 0.001) {
        restLon.current = v.centerLon;
        saveLon(-v.centerLon);
      }
      return;
    }
    if (cancelled) return;
    const r = e.currentTarget.getBoundingClientRect();
    const norad = pick(e.clientX - r.left, e.clientY - r.top);
    if (norad !== null) onSelect(norad);
  };

  const { w: sw, h: sh } = sizeRef.current;
  const zoomed = view.zoom > ZOOM_MIN + 0.01;

  return (
    <div className={dragging ? 'sat-map sat-map--drag' : 'sat-map'} ref={wrap}>
      <canvas
        ref={canvas}
        title={t('sat.map.drag')}
        style={{ touchAction: zoomed ? 'none' : 'pan-y' }}
        onPointerDown={(e) => {
          if (e.pointerType === 'mouse' && e.button !== 0) return;
          pointers.current.set(e.pointerId, { x: e.clientX, y: e.clientY });
          try {
            e.currentTarget.setPointerCapture(e.pointerId);
          } catch {
            /* il puntatore e' gia' finito */
          }
          if (pointers.current.size === 2) {
            // due dita: pizzico, il trascinamento a un dito si annulla
            drag.current = null;
            setDragging(false);
            const [a, b] = [...pointers.current.values()];
            if (a && b) {
              ctl.manual();
              pinch.current = { d0: Math.max(1, Math.hypot(a.x - b.x, a.y - b.y)), view0: viewRef.current };
            }
            return;
          }
          if (pointers.current.size > 2) return;
          drag.current = { id: e.pointerId, x0: e.clientX, y0: e.clientY, view0: viewRef.current, moved: false };
        }}
        onPointerMove={(e) => {
          if (pointers.current.has(e.pointerId)) pointers.current.set(e.pointerId, { x: e.clientX, y: e.clientY });
          const pz = pinch.current;
          if (pz) {
            const [a, b] = [...pointers.current.values()];
            if (!a || !b) return;
            const r = e.currentTarget.getBoundingClientRect();
            const sc = r.width > 0 ? sw / r.width : 1;
            const d = Math.hypot(a.x - b.x, a.y - b.y);
            ctl.commit(zoomAt(pz.view0, d / pz.d0, ((a.x + b.x) / 2 - r.left) * sc, ((a.y + b.y) / 2 - r.top) * sc, sw, sh));
            return;
          }
          const d = drag.current;
          if (!d || d.id !== e.pointerId) return;
          const dx = e.clientX - d.x0;
          const dy = e.clientY - d.y0;
          if (!d.moved) {
            if (Math.hypot(dx, dy) < DRAG_PX) return;
            d.moved = true;
            ctl.manual();
            // se la mappa si stava ancora muovendo, il trascinamento riparte dalla vista di adesso
            d.view0 = viewRef.current;
            setDragging(true);
          }
          // segue il puntatore senza ritardo: nessuna animazione, il disegno e' agganciato al fotogramma
          ctl.commit(panView(d.view0, dx, dy, sw, sh));
        }}
        onPointerUp={(e) => endDrag(e, false)}
        onPointerCancel={(e) => endDrag(e, true)}
        onDoubleClick={() => {
          ctl.manual();
          restLon.current = 0;
          saveLon(0);
          ctl.animateTo(FULL_VIEW);
        }}
      />
      {alerts.length > 0 ? <AlertLayer width={width} layerRef={layer} observer={observer} positions={positions} alerts={alerts} /> : null}
      {countryList.length > 0 ? <CountryPicker list={countryList} mine={myCountry} onPick={goCountry} /> : null}
      <div className="sat-zoom">
        <button type="button" aria-label={t('sat.map.zoomIn')} title={t('sat.map.zoomIn')} disabled={view.zoom >= ZOOM_MAX - 0.001} onClick={() => buttonZoom(BUTTON_ZOOM)}>
          +
        </button>
        <button type="button" aria-label={t('sat.map.zoomOut')} title={t('sat.map.zoomOut')} disabled={!zoomed} onClick={() => buttonZoom(1 / BUTTON_ZOOM)}>
          −
        </button>
        <button
          type="button"
          aria-label={t('sat.map.zoomFit')}
          title={t('sat.map.zoomFit')}
          disabled={!zoomed}
          onClick={() => {
            ctl.manual();
            ctl.animateTo(restView());
          }}
        >
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" />
          </svg>
        </button>
      </div>
      <span className="sat-legend">
        <i className="sat-dot sat-dot--sun" /> {t('sat.sun')} <i className="sat-dot sat-dot--moon" /> {t('sat.moon')}
      </span>
      <span className="sat-perf" ref={perfEl} hidden aria-hidden="true" />
    </div>
  );
}

interface PickerProps {
  list: readonly CountryEntry[];
  mine: CountryEntry | null;
  onPick(index: number): void;
}

/** Il selettore dei paesi (centinaia di voci): e' un componente a parte che non si rirenderizza finche' l'elenco e la lingua non cambiano, quindi non costa nulla mentre la mappa si muove. */
const CountryPicker = memo(function CountryPicker({ list, mine, onPick }: PickerProps) {
  const t = useT();
  return (
    <select
      className="sat-country"
      aria-label={t('sat.map.country')}
      title={t('sat.map.country')}
      value=""
      onChange={(e) => {
        if (e.target.value) onPick(Number(e.target.value.slice(2)));
      }}
    >
      <option value="">{t('sat.map.country')}</option>
      {mine ? <option value={`i:${mine.index}`}>{t('sat.map.myCountry', { name: mine.name })}</option> : null}
      {list.map((c) => (
        <option key={c.index} value={`i:${c.index}`}>
          {c.name}
        </option>
      ))}
    </select>
  );
});

const PHASE_ORDER: Record<AlertPhase, number> = { flash: 0, pre: 1, live: 2, fade: 3 };

interface LayerProps {
  width: number;
  layerRef: RefObject<SVGSVGElement | null>;
  observer: { lat: number; lon: number } | null;
  positions: readonly SatPosition[];
  alerts: readonly AlertView[];
}

/** Le tre copie del mondo che possono toccare lo schermo (il loro `transform` e la visibilita' li decide `placeAlerts`, a ogni disegno). */
const COPIES = [-1, 0, 1] as const;

/**
 * Anelli e scritte degli avvisi sopra la mappa: un livello SVG che non prende i clic e segue la stessa vista della mappa (zoom, spostamento, copie affiancate).
 * La posizione dei gruppi non e' una proprieta' di React: la scrive `placeAlerts` dentro il disegno, quindi la mappa in movimento non rirenderizza il livello.
 */
const AlertLayer = memo(function AlertLayer({ width: w, layerRef, observer, positions, alerts }: LayerProps) {
  const t = useT();
  const h = Math.round(w / 2);
  const labelOf = (a: AlertView): string | null => {
    if (a.phase === 'pre') return a.secondsToAos > 0 ? t('sat.alert.pre', { name: a.name, t: mmss(a.secondsToAos) }) : t('sat.alert.soon', { name: a.name });
    if (a.phase === 'fade') return t('sat.alert.los', { name: a.name });
    return null;
  };
  const top = [...alerts].sort((a, b) => PHASE_ORDER[a.phase] - PHASE_ORDER[b.phase])[0];
  const flashing = alerts.filter((a) => a.phase === 'flash');
  return (
    <svg ref={layerRef} className="sat-alert-layer" width={w} height={h} viewBox={`0 0 ${w} ${h}`} aria-hidden="true">
      {observer && top
        ? COPIES.map((k) => (
            <g key={`st-${k}-${top.phase}`} data-lat={observer.lat} data-lon={observer.lon} data-k={k} visibility="hidden" className={`sat-ring sat-ring--${top.phase}`}>
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
        const label = labelOf(a);
        return COPIES.map((k) => (
          <g key={`${a.norad}-${k}-${a.phase}`} data-lat={p.lat_deg} data-lon={p.lon_deg} data-k={k} visibility="hidden" className={`sat-ring sat-ring--${a.phase}`}>
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
});
