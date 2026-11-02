import { useEffect, useState } from 'react';
import { feature } from 'topojson-client';
import type { GeometryCollection, Topology } from 'topojson-specification';

type Ring = [number, number][];

/** Quanto lontano (in gradi) si prendono le isole vicine al punto quando nessun territorio lo contiene. */
const NEAR_DEG = 1.5;

/**
 * La forma del paese (solo il contorno) di un punto: il territorio che lo contiene o, per le isole piccole e i punti in mare, le terre
 * entro 1,5 gradi. I contorni vengono dai confini a 1:10 milioni di Natural Earth (`world-atlas`, caricati solo quando servono).
 */
export function CountryShape({ lat, lon, label }: { lat: number; lon: number; label: string }) {
  const [rings, setRings] = useState<Ring[] | null>(null);
  useEffect(() => {
    let alive = true;
    void import('world-atlas/countries-10m.json').then((m) => {
      if (alive) setRings(shapeAround(m.default as unknown as Topology, lat, lon));
    });
    return () => {
      alive = false;
    };
  }, [lat, lon]);
  if (!rings || rings.length === 0) return null;
  const k = Math.cos((lat * Math.PI) / 180) || 1;
  const pts = rings.flat().map(([x, y]) => [x * k, -y] as const);
  const xs = pts.map((p) => p[0]);
  const ys = pts.map((p) => p[1]);
  const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  const size = Math.max(x1 - x0, y1 - y0) || 0.1;
  const pad = size * 0.08;
  const view = `${x0 - pad} ${y0 - pad} ${size + 2 * pad} ${size + 2 * pad}`;
  const d = rings.map((r) => 'M' + r.map(([x, y]) => `${(x * k).toFixed(4)},${(-y).toFixed(4)}`).join('L') + 'Z').join('');
  return (
    <figure className="dxd-shape" aria-label={label}>
      <svg viewBox={view} preserveAspectRatio="xMidYMid meet" role="img">
        <path d={d} className="dxd-shape-land" vectorEffect="non-scaling-stroke" fillRule="evenodd" />
        <circle cx={lon * k} cy={-lat} r={size * 0.03} className="dxd-shape-dot" />
      </svg>
      <figcaption>{label}</figcaption>
    </figure>
  );
}

/** I contorni del territorio che contiene il punto, oppure delle terre vicine. */
export function shapeAround(topology: Topology, lat: number, lon: number): Ring[] {
  const fc = feature(topology, topology.objects.countries as GeometryCollection);
  const polygons: Ring[][] = [];
  for (const f of fc.features) {
    const g = f.geometry;
    if (!g) continue;
    if (g.type === 'Polygon') polygons.push(g.coordinates as Ring[]);
    if (g.type === 'MultiPolygon') for (const p of g.coordinates) polygons.push(p as Ring[]);
  }
  const inside = polygons.find((p) => p[0] !== undefined && contains(p[0], lon, lat));
  if (inside) return inside;
  const near = polygons.filter((p) => (p[0] ?? []).some(([x, y]) => Math.abs(x - lon) < NEAR_DEG && Math.abs(y - lat) < NEAR_DEG));
  return near.flat();
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
