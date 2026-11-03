import { useEffect, useState } from 'react';
import { geoArea, geoEquirectangular, geoGraticule10, geoMercator, geoPath } from 'd3-geo';
import { feature } from 'topojson-client';
import type { Feature, FeatureCollection, Geometry, MultiPolygon } from 'geojson';
import type { GeometryCollection, Topology } from 'topojson-specification';

type Ring = [number, number][];

/** Quanto lontano (in gradi) si prendono le isole vicine al punto quando nessun territorio lo contiene. */
const NEAR_DEG = 1.5;
/** Dimensioni del disegno (unita' del viewBox). */
const W = 260;
const H = 180;
/** La regione mostra almeno questi gradi di longitudine (isole piccole) e al massimo questi (paesi molto grandi). */
const MIN_SPAN_DEG = 1.5;
const MAX_SPAN_DEG = 200;
/** Le altre parti dello stesso paese entro questi gradi entrano nella regione (arcipelaghi, isole maggiori). */
const GROUP_DEG = 6;
/** Il mondo in miniatura in basso a destra. */
const MINI_W = 84;
const MINI_H = 42;

interface Drawing {
  graticule: string;
  land: string;
  country: string;
  pin: [number, number];
  world: string;
  frame: { x: number; y: number; w: number; h: number };
}

/**
 * Il riquadro del paese nella scheda DXpedition: la regione attorno al paese (Mercatore ruotato sulla longitudine del paese, cosi'
 * funziona anche vicino all'antimeridiano) con il paese in verde e il punto rosso sulla DXpedition, e in basso a destra il mondo in
 * miniatura con un riquadro rosso su dove si trova. Il paese e' il territorio che contiene il punto o, per le isole piccole e i punti
 * in mare, le terre entro 1,5 gradi. Confini a 1:10 milioni e terre a 1:110 milioni di Natural Earth (`world-atlas`, caricati solo
 * quando servono).
 */
export function CountryShape({ lat, lon, label, ariaLabel, onReady }: { lat: number; lon: number; label: string; ariaLabel?: string; onReady?(): void }) {
  // il disegno porta con se' il punto per cui e' stato fatto: cambiando DXpedition non si mostra mai quello della precedente
  const [done, setDone] = useState<{ key: string; drawing: Drawing } | null>(null);
  const key = `${lat},${lon}`;
  useEffect(() => {
    let alive = true;
    void loadAtlas().then(([big, small]) => {
      if (alive) setDone({ key, drawing: draw(big, small, lat, lon) });
    });
    return () => {
      alive = false;
    };
  }, [key, lat, lon]);
  const drawing = done?.key === key ? done.drawing : null;
  const ready = drawing !== null;
  useEffect(() => {
    if (ready) onReady?.();
  }, [ready, onReady]);
  if (!drawing) return <figure className="dxd-shape dxd-shape--loading" aria-busy="true" aria-label={ariaLabel ?? label} />;
  const f = drawing.frame;
  return (
    <figure className="dxd-shape">
      <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label={ariaLabel ?? label}>
        <rect width={W} height={H} className="dxd-shape-sea" />
        <path d={drawing.graticule} className="dxd-shape-grat" />
        <path d={drawing.land} className="dxd-shape-land" />
        <path d={drawing.country} className="dxd-shape-country" />
        <circle cx={drawing.pin[0]} cy={drawing.pin[1]} r={4} className="dxd-shape-pin" />
        <g transform={`translate(${W - MINI_W - 6},${H - MINI_H - 6})`}>
          <rect width={MINI_W} height={MINI_H} className="dxd-shape-mini" />
          <path d={drawing.world} className="dxd-shape-land" />
          <rect x={f.x} y={f.y} width={f.w} height={f.h} className="dxd-shape-frame" />
        </g>
      </svg>
      <figcaption>{label}</figcaption>
    </figure>
  );
}

let atlas: Promise<[Topology, Topology]> | null = null;

/** I confini e le terre di Natural Earth, caricati una volta sola (si possono chiedere in anticipo, all'apertura della scheda). */
export function loadAtlas(): Promise<[Topology, Topology]> {
  atlas ??= Promise.all([import('world-atlas/countries-10m.json'), import('world-atlas/land-110m.json')]).then(
    ([big, small]) => [big.default as unknown as Topology, small.default as unknown as Topology],
  );
  return atlas;
}

/** Prepara i tracciati della regione e del mondo in miniatura. */
export function draw(countries: Topology, world: Topology, lat: number, lon: number): Drawing {
  const target = targetAround(countries, lat, lon);
  const proj = geoMercator().rotate([-lon, 0]).scale(1).translate([0, 0]);
  const unit = geoPath(proj);
  const point: Feature = { type: 'Feature', properties: {}, geometry: { type: 'Point', coordinates: [lon, lat] } };
  const [[x0, y0], [x1, y1]] = unit.bounds({ type: 'FeatureCollection', features: [target.zoom, point] } as FeatureCollection);
  const deg = Math.PI / 180;
  let s = Math.min((W * 0.7) / Math.max(x1 - x0, 1e-6), (H * 0.7) / Math.max(y1 - y0, 1e-6));
  s = Math.min(s, W / (MIN_SPAN_DEG * deg));
  s = Math.max(s, W / (MAX_SPAN_DEG * deg));
  // il centro: quello del paese, ma il punto della DXpedition deve restare dentro
  let cx = (x0 + x1) / 2;
  let cy = (y0 + y1) / 2;
  const [px, py] = proj([lon, lat]) ?? [0, 0];
  const mx = (W / 2 - 12) / s;
  const my = (H / 2 - 12) / s;
  cx = Math.min(Math.max(cx, px - mx), px + mx);
  cy = Math.min(Math.max(cy, py - my), py + my);
  proj.scale(s).translate([W / 2 - s * cx, H / 2 - s * cy]).clipExtent([[-2, -2], [W + 2, H + 2]]);
  const path = geoPath(proj);
  const land = rewound(merged(feature(countries, countries.objects.land as GeometryCollection)));
  const pin = proj([lon, lat]) ?? [W / 2, H / 2];

  const mini = geoEquirectangular().fitExtent([[2, 2], [MINI_W - 2, MINI_H - 2]], { type: 'Sphere' });
  const miniPath = geoPath(mini);
  const worldLand = feature(world, world.objects.land as GeometryCollection);
  // il riquadro copre la parte di mondo che si vede nella regione
  const spanLon = W / s / deg;
  const top = proj.invert?.([0, 0]);
  const bottom = proj.invert?.([0, H]);
  const spanLat = top && bottom ? Math.abs(top[1] - bottom[1]) : spanLon * (H / W);
  const k = (MINI_W - 4) / 360;
  const [fx, fy] = mini([lon, lat]) ?? [0, 0];
  const fw = Math.max(spanLon * k, 6);
  const fh = Math.max(spanLat * k, 4);
  return {
    graticule: path(geoGraticule10()) ?? '',
    land: path(land) ?? '',
    country: path(rewound(target.country)) ?? '',
    pin: [pin[0], pin[1]],
    world: miniPath(worldLand) ?? '',
    frame: { x: fx - fw / 2, y: fy - fh / 2, w: fw, h: fh },
  };
}

/** Il paese da colorare (tutto il territorio) e la parte su cui centrare la regione (il pezzo che contiene il punto con le parti vicine, o le isole vicine). */
function targetAround(topology: Topology, lat: number, lon: number): { country: Feature; zoom: Feature } {
  const fc = feature(topology, topology.objects.countries as GeometryCollection);
  for (const f of fc.features) {
    const inside = polygonsOf(f.geometry).find((p) => p[0] !== undefined && contains(p[0], lon, lat));
    if (inside) {
      const group = polygonsOf(f.geometry).filter((p) => p === inside || (p[0] ?? []).some(([x, y]) => Math.abs(x - lon) < GROUP_DEG && Math.abs(y - lat) < GROUP_DEG));
      return { country: f, zoom: asFeature(group) };
    }
  }
  const near = nearPolygons(fc.features, lat, lon);
  const shape = asFeature(near);
  return { country: shape, zoom: shape };
}

/**
 * Alcuni anelli dei confini a 1:10 milioni (le Maldive) sono girati al contrario o degeneri: per d3-geo coprirebbero tutto il mondo tranne
 * l'isola. Un poligono piu' grande di mezza sfera si rigira.
 */
function rewound(f: Feature): Feature {
  const polygons = polygonsOf(f.geometry);
  if (polygons.length === 0) return f;
  const big = (p: Ring[]) => geoArea({ type: 'Polygon', coordinates: p }) > 2 * Math.PI;
  // gli anelli degeneri (tre punti quasi allineati) restano enormi anche rigirati: si lasciano fuori
  const fixed = polygons.map((p) => (big(p) ? p.map((r) => [...r].reverse()) : p)).filter((p) => !big(p));
  return asFeature(fixed);
}

/** Le terre arrivano come una raccolta con un solo elemento: tutti i poligoni in uno. */
function merged(fc: FeatureCollection): Feature {
  return asFeature(fc.features.flatMap((f) => polygonsOf(f.geometry)));
}

function asFeature(polygons: Ring[][]): Feature {
  const geometry: MultiPolygon = { type: 'MultiPolygon', coordinates: polygons };
  return { type: 'Feature', properties: {}, geometry };
}

function polygonsOf(g: Geometry | null): Ring[][] {
  if (!g) return [];
  if (g.type === 'Polygon') return [g.coordinates as Ring[]];
  if (g.type === 'MultiPolygon') return g.coordinates as Ring[][];
  return [];
}

function nearPolygons(features: Feature[], lat: number, lon: number): Ring[][] {
  return features
    .flatMap((f) => polygonsOf(f.geometry))
    .filter((p) => (p[0] ?? []).some(([x, y]) => Math.abs(x - lon) < NEAR_DEG && Math.abs(y - lat) < NEAR_DEG));
}

/** I contorni del territorio che contiene il punto, oppure delle terre vicine. */
export function shapeAround(topology: Topology, lat: number, lon: number): Ring[] {
  const fc = feature(topology, topology.objects.countries as GeometryCollection);
  const polygons = fc.features.flatMap((f) => polygonsOf(f.geometry));
  const inside = polygons.find((p) => p[0] !== undefined && contains(p[0], lon, lat));
  if (inside) return inside;
  return nearPolygons(fc.features, lat, lon).flat();
}

function contains(ring: Ring, x: number, y: number): boolean {
  let inside = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const a = ring[i];
    const b = ring[j];
    if (!a || !b) continue;
    const [xi, yi] = a;
    const [xj, yj] = b;
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) inside = !inside;
  }
  return inside;
}
