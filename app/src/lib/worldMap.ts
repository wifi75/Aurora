import { feature, mesh } from 'topojson-client';
import type { GeometryObject, Topology } from 'topojson-specification';
import countries from 'world-atlas/countries-110m.json';

type Coordinates = number[][];

function line(points: Coordinates, close: boolean): string {
  let d = '';
  points.forEach((p, i) => {
    const x = ((p[0] ?? 0) + 180).toFixed(2);
    const y = (90 - (p[1] ?? 0)).toFixed(2);
    d += `${i === 0 ? 'M' : 'L'}${x} ${y}`;
  });
  return close ? `${d}Z` : d;
}

function geometryPath(g: { type: string; coordinates?: unknown; geometries?: unknown[] }): string {
  switch (g.type) {
    case 'Polygon':
      return (g.coordinates as Coordinates[]).map((r) => line(r, true)).join('');
    case 'MultiPolygon':
      return (g.coordinates as Coordinates[][]).map((poly) => poly.map((r) => line(r, true)).join('')).join('');
    case 'LineString':
      return line(g.coordinates as Coordinates, false);
    case 'MultiLineString':
      return (g.coordinates as Coordinates[]).map((l) => line(l, false)).join('');
    case 'GeometryCollection':
      return (g.geometries as Array<{ type: string; coordinates?: unknown }>).map(geometryPath).join('');
    default:
      return '';
  }
}

export interface WorldPaths {
  /** La terra, da riempire. */
  land: string;
  /** I confini tra gli stati, da tracciare. */
  borders: string;
}

let cached: WorldPaths | null = null;

/** Il mondo come tracciati SVG nelle coordinate della mappa (0-360 x 0-180). Si calcola una volta. */
export function worldPaths(): WorldPaths {
  if (cached) return cached;
  const topology = countries as unknown as Topology;
  const landObject = topology.objects['land'] as GeometryObject;
  const countryObject = topology.objects['countries'] as GeometryObject;
  const land = feature(topology, landObject);
  const landGeometries = 'features' in land ? land.features.map((f) => f.geometry) : [land.geometry];
  cached = {
    land: landGeometries.map((g) => geometryPath(g as { type: string; coordinates?: unknown })).join(''),
    borders: geometryPath(mesh(topology, countryObject, (a, b) => a !== b) as unknown as { type: string; coordinates?: unknown }),
  };
  return cached;
}
