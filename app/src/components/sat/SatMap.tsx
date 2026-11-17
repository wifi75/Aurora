import { useEffect, useMemo, useRef, useState } from 'react';
import { geoEquirectangular, geoPath } from 'd3-geo';
import { feature } from 'topojson-client';
import type { FeatureCollection } from 'geojson';
import type { GeometryCollection, Topology } from 'topojson-specification';
import type { SatPosition } from '../../api/client';
import { useLang, useT } from '../../i18n';
import { footprintPolygon, mmss, nightAlpha, sunAltitudeDeg, type LatLon } from '../../lib/satGeo';
import type { AlertPhase, AlertView } from '../../lib/satAlerts';
import { buildCountryList, countryAt, countryView } from '../../lib/satCountries';
import { NIGHT_MAX_LEVEL, PHOTO_FULL, photoLevel, terraUrls } from '../../lib/satMapChoice';
import { DEFAULT_ZOOM_MODE, FULL_VIEW, ZOOM_MAX, ZOOM_MIN, autoView, clampView, lerpView, panView, projectView, projectViewRaw, viewsClose, zoomAt, type MapView, type ZoomMode } from '../../lib/satView';

interface Props {
  /** L'immagine della mappa del mondo (equirettangolare); senza, la mappa inclusa in Aurora (terre e confini di Natural Earth). */
  mapUrl: string | null;
  /** La Terra della NASA (foto di giorno, luci delle citta' di notte); ha la precedenza su `mapUrl`. Finche' le foto caricano resta la mappa vettoriale. */
  terra?: boolean;
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
/** Costante di tempo dell'animazione della vista (ms): a regime (99%) in circa 650 ms. */
const TAU_MS = 140;
/** Di quanto cambia lo zoom a ogni clic sui pulsanti «+» e «−». */
const BUTTON_ZOOM = 1.6;

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

/** Una foto decodificata: bitmap (decodificata fuori dal filo principale) o, dove manca `createImageBitmap`, un elemento immagine. */
type Photo = ImageBitmap | HTMLImageElement;

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

/** La foto ridotta a `level` pixel di larghezza (la meta' in altezza), a gradini dimezzando: meglio di un solo salto. Le ridotte restano in `cache`. */
function photoAt(cache: Map<number, HTMLCanvasElement>, source: Photo, level: number): CanvasImageSource {
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
  return c;
}

/** Le foto della Terra ridotte per lo zoom e la notte gia' mascherata dal Sole (si rifa' quando il Sole si e' mosso di un decimo di grado). */
interface PhotoCache {
  photos: { day: Photo; night: Photo } | null;
  day: Map<number, HTMLCanvasElement>;
  nightBase: Map<number, HTMLCanvasElement>;
  sunKey: string;
  nightMasked: Map<number, HTMLCanvasElement>;
}

const PALETTE = { grid: 'rgba(255,255,255,0.16)', label: 'rgba(255,255,255,0.75)', past: 'rgba(255,255,255,0.55)', future: '#ffd84d', foot: '#3ddc84', sat: '#ffffff', sel: '#ffd84d' };

/** La mappa dei satelliti: mondo con giorno e notte, Sole e Luna, la stazione, i satelliti con la loro impronta radio e la traccia del satellite scelto. Si sposta trascinandola, si ingrandisce con la rotella, il pizzico o i pulsanti e gira senza bordi; con un satellite in vista si inquadra da sola. */
export function SatMap({ mapUrl, terra = false, borders = false, sun, moon, observer, stationLabel, positions, selected, track, trackNowIndex, alerts = [], zoomMode = DEFAULT_ZOOM_MODE, onSelect }: Props) {
  const t = useT();
  const lang = useLang((st) => st.lang);
  const wrap = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
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
  const viewRef = useRef(view);
  const sizeRef = useRef({ w: 900, h: 450 });
  const [dragging, setDragging] = useState(false);
  const drag = useRef<{ id: number; x0: number; y0: number; view0: MapView; moved: boolean } | null>(null);
  const pointers = useRef(new Map<number, { x: number; y: number }>());
  const pinch = useRef<{ d0: number; view0: MapView } | null>(null);
  // animazione della vista verso una destinazione
  const goal = useRef<MapView | null>(null);
  const raf = useRef<number | null>(null);
  const lastTick = useRef(0);
  // zoom automatico: se ha inquadrato il passaggio, se l'utente l'ha sospeso con un gesto, se un satellite e' in vista
  const engaged = useRef(false);
  const suspended = useRef(false);
  const inPass = useRef(false);

  sizeRef.current = { w: width, h: Math.round(width / 2) };

  const commit = (v: MapView) => {
    viewRef.current = v;
    setView(v);
  };
  const stopAnim = () => {
    if (raf.current !== null) unschedule(raf.current);
    raf.current = null;
    goal.current = null;
  };
  const step = (now: number) => {
    raf.current = null;
    const dest = goal.current;
    if (!dest) return;
    const dt = Math.max(0, Math.min(100, now - lastTick.current));
    lastTick.current = now;
    const cur = viewRef.current;
    const next = viewsClose(cur, dest) ? dest : clampView(lerpView(cur, dest, 1 - Math.exp(-dt / TAU_MS)));
    commit(next);
    if (next === dest) {
      goal.current = null;
      return;
    }
    raf.current = schedule(step);
  };
  /** Porta la vista a `dest` con un movimento morbido (subito, con «riduci animazioni»). */
  const animateTo = (dest: MapView) => {
    const d = clampView(dest);
    if (reducedMotion()) {
      stopAnim();
      commit(d);
      return;
    }
    goal.current = d;
    if (raf.current === null) {
      lastTick.current = performance.now();
      raf.current = schedule(step);
    }
  };
  /** Un gesto dell'utente sulla mappa: ferma l'animazione e, se c'e' un satellite in vista, sospende lo zoom automatico fino al prossimo AOS. */
  const manual = () => {
    stopAnim();
    engaged.current = false;
    if (inPass.current) suspended.current = true;
  };
  const restView = (): MapView => clampView({ zoom: 1, centerLat: 0, centerLon: restLon.current });
  const buttonZoom = (factor: number) => {
    const { w, h } = sizeRef.current;
    const from = goal.current ?? viewRef.current;
    const dest = zoomAt(from, factor, w / 2, h / 2, w, h);
    manual();
    animateTo(dest);
  };

  useEffect(() => stopAnim, []); // eslint-disable-line react-hooks/exhaustive-deps

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
    const urls = terraUrls();
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
  }, [terra]);
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
      animateTo(desired);
      return;
    }
    // nessun satellite in vista (LOS) o zoom spento: se la vista l'ha decisa lo zoom automatico, si torna intera
    if (engaged.current) {
      engaged.current = false;
      animateTo(restView());
    }
    if (!anyLive) suspended.current = false;
    // la vista si rinnova solo quando cambia davvero (la stazione e le posizioni arrivano come nuovi oggetti a ogni aggiornamento)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [desiredKey, anyLive]);

  // la rotella ingrandisce intorno al cursore; l'ascoltatore e' diretto perche' deve poter fermare lo scorrimento della pagina
  useEffect(() => {
    const c = canvas.current;
    if (!c) return;
    const onWheel = (e: WheelEvent) => {
      const cur = viewRef.current;
      const dy = e.deltaMode === 1 ? e.deltaY * 16 : e.deltaMode === 2 ? e.deltaY * 100 : e.deltaY;
      const factor = Math.exp(-dy * 0.0016);
      // a vista intera la rotella verso il basso lascia scorrere la pagina
      if (factor < 1 && cur.zoom <= ZOOM_MIN + 1e-6) return;
      e.preventDefault();
      const r = c.getBoundingClientRect();
      const { w, h } = sizeRef.current;
      const sc = r.width > 0 ? w / r.width : 1;
      manual();
      commit(zoomAt(cur, factor, (e.clientX - r.left) * sc, (e.clientY - r.top) * sc, w, h));
    };
    c.addEventListener('wheel', onWheel, { passive: false });
    return () => c.removeEventListener('wheel', onWheel);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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
        const a = nightAlpha(sunAltitudeDeg(lat, lon, sun.lat, sun.lon), photoReady ? 0.92 : 0.74);
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

  // l'elenco dei paesi nella lingua dell'interfaccia e il paese della stazione
  const countryList = useMemo(() => (countries ? buildCountryList(countries, lang) : []), [countries, lang]);
  const myCountry = useMemo(() => {
    if (!countries || !observer) return null;
    const i = countryAt(countries, observer.lat, observer.lon);
    return i === null ? null : (countryList.find((c) => c.index === i) ?? null);
  }, [countries, countryList, observer]);
  const goCountry = (value: string) => {
    const i = Number(value.slice(2));
    const c = countryList.find((x) => x.index === i);
    if (!c) return;
    manual();
    animateTo(countryView(c.box));
  };

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

    // la vista: con zoom z il mondo e' largo W = w*z e alto H = h*z; (left, top) e' l'angolo in alto a sinistra della copia centrale
    const z = view.zoom;
    const W = w * z;
    const H = h * z;
    const left = Math.round(w / 2 - ((view.centerLon + 180) / 360) * W);
    const top = Math.round(h / 2 - ((90 - view.centerLat) / 180) * H);
    hitW.current = W;
    /** La proiezione con zoom e spostamento: i simboli si disegnano qui, sempre della stessa grandezza in pixel. */
    const pr = (lat: number, lon: number) => projectView(lat, lon, w, h, view);
    /** Come `pr` ma senza riportare la longitudine, per le linee con le longitudini srotolate. */
    const prU = (lat: number, lon: number) => projectViewRaw(lat, lon, w, h, view);
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
    /** Disegna la base: l'origine e' l'angolo in alto a sinistra della mappa e il disegno e' ingrandito di z (mappa, notte). */
    const eachWorld = (fn: () => void) =>
      eachCopy(left, left + W, () => {
        ctx.translate(left, top);
        ctx.scale(z, z);
        fn();
      });

    // sotto a tutto il mare scuro (copre le fessure fra le copie e il tempo del caricamento)
    const g = ctx.createLinearGradient(0, 0, 0, h);
    g.addColorStop(0, '#0b1a33');
    g.addColorStop(1, '#0a2440');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, w, h);

    /** Il livello vettoriale (terre e confini, o solo i confini) alla risoluzione che serve allo zoom, fino a 4096 px. */
    const vectorLayer = (cache: { current: VectorBase }, bordersOnly: boolean): HTMLCanvasElement | null => {
      if (!countries) return null;
      const bucket = z <= 1 ? 1 : z <= 2 ? 2 : z <= 4 ? 4 : 8;
      let res = bucket;
      while (res > 1 && w * dpr * res > MAX_BASE_PX) res /= 2;
      const b = cache.current;
      if (!b || b.w !== w || b.dpr !== dpr || b.bucket !== bucket || b.res !== res) cache.current = { w, dpr, bucket, res, canvas: drawBuiltin(countries, w, h, dpr * res, (bordersOnly ? 0.5 : 0.65) / bucket, bordersOnly) };
      return cache.current?.canvas ?? null;
    };

    // le foto ridotte restano in memoria finche' sono le stesse; il Sole mosso di un decimo di grado rifa' la notte
    const pc = photoCache.current;
    if (pc.photos !== (photoReady ? photos : null)) {
      pc.photos = photoReady ? photos : null;
      pc.day.clear();
      pc.nightBase.clear();
      pc.nightMasked.clear();
    }
    const sunKey = `${Math.round(sun.lat * 10)},${Math.round(sun.lon * 10)}`;
    if (pc.sunKey !== sunKey) {
      pc.sunKey = sunKey;
      pc.nightMasked.clear();
    }
    const wanted = photoLevel(w * dpr * z);

    // fondo: la Terra della NASA, la mappa dell'utente o la mappa semplice
    ctx.imageSmoothingEnabled = true;
    ctx.imageSmoothingQuality = 'high';
    let veil = true;
    if (photoReady && photos) {
      const day = wanted >= PHOTO_FULL ? photos.day : photoAt(pc.day, photos.day, wanted);
      eachWorld(() => ctx.drawImage(day, 0, 0, w, h));
      veil = false;
    } else if (image) eachWorld(() => ctx.drawImage(image, 0, 0, w, h));
    else {
      const bc = vectorLayer(base, false);
      if (bc) eachWorld(() => ctx.drawImage(bc, 0, 0, w, h));
    }

    // la notte: con la Terra la foto delle luci mascherata dal Sole (bordo morbido), altrimenti un velo blu scuro
    if (!veil && photos && night) {
      const level = Math.min(wanted, NIGHT_MAX_LEVEL);
      let lights = pc.nightMasked.get(level);
      if (!lights) {
        const c2 = document.createElement('canvas');
        c2.width = level;
        c2.height = Math.round(level / 2);
        const x = c2.getContext('2d');
        if (x) {
          x.imageSmoothingEnabled = true;
          x.imageSmoothingQuality = 'high';
          x.drawImage(photoAt(pc.nightBase, photos.night, level), 0, 0, c2.width, c2.height);
          x.globalCompositeOperation = 'destination-in';
          x.drawImage(night, 0, 0, c2.width, c2.height);
          pc.nightMasked.set(level, c2);
          lights = c2;
        }
      }
      const lt = lights;
      if (lt) eachWorld(() => ctx.drawImage(lt, 0, 0, w, h));
    } else if (night) eachWorld(() => ctx.drawImage(night, 0, 0, w, h));

    // i confini sottili sopra la foto (opzionali)
    if (borders && photoReady) {
      const bl = vectorLayer(bordersBase, true);
      if (bl) eachWorld(() => ctx.drawImage(bl, 0, 0, w, h));
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

    /** Una linea continua (longitudini srotolate) ripetuta in tutte le copie che la mostrano; con `fill` anche riempita. */
    const drawLine = (pts: readonly LatLon[], fill: boolean) => {
      if (pts.length < 2) return;
      const u = unwrap(pts);
      let lo = Infinity;
      let hi = -Infinity;
      for (const p of u) {
        const x = prU(p[0], p[1])[0];
        lo = Math.min(lo, x);
        hi = Math.max(hi, x);
      }
      eachCopy(lo, hi, () => {
        ctx.beginPath();
        u.forEach((p, i) => {
          const [x, y] = prU(p[0], p[1]);
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
      const [x, y] = pr(p.lat_deg, p.lon_deg);
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
      const [x, y] = pr(observer.lat, observer.lon);
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
    const [sx, sy] = pr(sun.lat, sun.lon);
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
    const [mx, my] = pr(moon.lat, moon.lon);
    eachCopy(mx - 8, mx + 8, () => {
      ctx.fillStyle = '#d9d9d9';
      ctx.shadowColor = '#fff';
      ctx.shadowBlur = 8;
      ctx.beginPath();
      ctx.arc(mx, my, 6, 0, Math.PI * 2);
      ctx.fill();
      ctx.shadowBlur = 0;
    });
  }, [width, image, countries, photos, photoReady, borders, night, view, sun.lat, sun.lon, moon.lat, moon.lon, observer, stationLabel, positions, selected, track, trackNowIndex]);

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
      // lo spostamento a mano della mappa intera si ricorda; da ingrandita no (e la vista automatica non si salva mai)
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
              manual();
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
            commit(zoomAt(pz.view0, d / pz.d0, ((a.x + b.x) / 2 - r.left) * sc, ((a.y + b.y) / 2 - r.top) * sc, sw, sh));
            return;
          }
          const d = drag.current;
          if (!d || d.id !== e.pointerId) return;
          const dx = e.clientX - d.x0;
          const dy = e.clientY - d.y0;
          if (!d.moved) {
            if (Math.hypot(dx, dy) < DRAG_PX) return;
            d.moved = true;
            manual();
            setDragging(true);
          }
          commit(panView(d.view0, dx, dy, sw, sh));
        }}
        onPointerUp={(e) => endDrag(e, false)}
        onPointerCancel={(e) => endDrag(e, true)}
        onDoubleClick={() => {
          manual();
          restLon.current = 0;
          saveLon(0);
          animateTo(FULL_VIEW);
        }}
      />
      {alerts.length > 0 ? <AlertLayer width={width} view={view} observer={observer} positions={positions} alerts={alerts} /> : null}
      {countryList.length > 0 ? (
        <select
          className="sat-country"
          aria-label={t('sat.map.country')}
          title={t('sat.map.country')}
          value=""
          onChange={(e) => {
            if (e.target.value) goCountry(e.target.value);
          }}
        >
          <option value="">{t('sat.map.country')}</option>
          {myCountry ? <option value={`i:${myCountry.index}`}>{t('sat.map.myCountry', { name: myCountry.name })}</option> : null}
          {countryList.map((c) => (
            <option key={c.index} value={`i:${c.index}`}>
              {c.name}
            </option>
          ))}
        </select>
      ) : null}
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
            manual();
            animateTo(restView());
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
    </div>
  );
}

const PHASE_ORDER: Record<AlertPhase, number> = { flash: 0, pre: 1, live: 2, fade: 3 };

interface LayerProps {
  width: number;
  view: MapView;
  observer: { lat: number; lon: number } | null;
  positions: readonly SatPosition[];
  alerts: readonly AlertView[];
}

/** Anelli e scritte degli avvisi sopra la mappa: un livello SVG che non prende i clic e segue la stessa vista della mappa (zoom, spostamento, copie affiancate come per il disegno). */
function AlertLayer({ width: w, view, observer, positions, alerts }: LayerProps) {
  const t = useT();
  const h = Math.round(w / 2);
  const worldW = w * view.zoom;
  /** Le posizioni x di un punto, con le copie del mondo che toccano lo schermo (`k` serve da chiave: non cambia mentre la mappa si muove). */
  const xs = (x: number): { k: number; x: number }[] => [-1, 0, 1].map((k) => ({ k, x: x + k * worldW })).filter((o) => o.x > -80 && o.x < w + 80);
  const labelOf = (a: AlertView): string | null => {
    if (a.phase === 'pre') return a.secondsToAos > 0 ? t('sat.alert.pre', { name: a.name, t: mmss(a.secondsToAos) }) : t('sat.alert.soon', { name: a.name });
    if (a.phase === 'fade') return t('sat.alert.los', { name: a.name });
    return null;
  };
  const top = [...alerts].sort((a, b) => PHASE_ORDER[a.phase] - PHASE_ORDER[b.phase])[0];
  const flashing = alerts.filter((a) => a.phase === 'flash');
  const st = observer ? projectView(observer.lat, observer.lon, w, h, view) : null;
  return (
    <svg className="sat-alert-layer" width={w} height={h} viewBox={`0 0 ${w} ${h}`} aria-hidden="true">
      {st && top
        ? xs(st[0]).map(({ k, x }) => (
            <g key={`st-${k}-${top.phase}`} transform={`translate(${x} ${st[1]})`} className={`sat-ring sat-ring--${top.phase}`}>
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
        const [px, py] = projectView(p.lat_deg, p.lon_deg, w, h, view);
        const label = labelOf(a);
        return xs(px).map(({ k, x }) => (
          <g key={`${a.norad}-${k}-${a.phase}`} transform={`translate(${x} ${py})`} className={`sat-ring sat-ring--${a.phase}`}>
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
