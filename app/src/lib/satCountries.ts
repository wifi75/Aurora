/*
 * I paesi della mappa dei satelliti: elenco per il selettore «Zoom su un paese…» (nome nella lingua dell'interfaccia),
 * riquadro di ognuno (anche a cavallo dell'antimeridiano), paese della stazione e vista che lo inquadra.
 * Solo logica: i dati sono le terre di Natural Earth gia' caricate per la mappa vettoriale.
 */
import { geoArea, geoBounds, geoContains } from 'd3-geo';
import type { Feature, FeatureCollection, Geometry } from 'geojson';
import { countryName } from './countryCodes';
import { fitBox, wrapLon, type GeoBox, type MapView } from './satView';

export interface CountryEntry {
  /** Posizione della terra nella collezione (unica: lo stesso codice ISO puo' comparire due volte). */
  index: number;
  name: string;
  box: GeoBox;
}

/** Margine e campo minimo dell'inquadratura di un paese: le isole minuscole non vanno ingrandite all'infinito. */
export const COUNTRY_FIT = { margin: 0.12, minSpanDeg: 8 } as const;

const boxes = new WeakMap<object, GeoBox | null>();

/** Il riquadro da `geoBounds` di d3 ([[ovest, sud], [est, nord]], con ovest > est se attraversa l'antimeridiano) come `GeoBox`. */
export function boxFromBounds(b: readonly [readonly [number, number], readonly [number, number]]): GeoBox | null {
  const [[w, s], [e, n]] = b;
  if (![w, s, e, n].every(Number.isFinite)) return null;
  const spanLon = e >= w ? e - w : e + 360 - w;
  return { south: s, north: n, west: wrapLon(w), spanLon: Math.min(360, spanLon) };
}

const mod360 = (x: number): number => ((x % 360) + 360) % 360;

/** La distanza in gradi fra due riquadri (la maggiore fra latitudine e longitudine sul cerchio); 0 se si toccano o si sovrappongono. */
export function boxGap(a: GeoBox, b: GeoBox): number {
  const lat = Math.max(0, b.south - a.north, a.south - b.north);
  let lon = 0;
  if (a.spanLon < 360 && b.spanLon < 360 && mod360(b.west - a.west) > a.spanLon && mod360(a.west - b.west) > b.spanLon) {
    lon = Math.min(mod360(b.west - (a.west + a.spanLon)), mod360(a.west - (b.west + b.spanLon)));
  }
  return Math.max(lat, lon);
}

/** L'unione di due riquadri: la longitudine si prende dal lato corto (l'intervallo vuoto piu' largo sul cerchio resta fuori). */
export function boxUnion(a: GeoBox, b: GeoBox): GeoBox {
  const south = Math.min(a.south, b.south);
  const north = Math.max(a.north, b.north);
  if (a.spanLon >= 360 || b.spanLon >= 360) return { south, north, west: -180, spanLon: 360 };
  // gli intervalli [west, west + span] su 0-360, spezzati se superano 360, poi uniti
  const parts: [number, number][] = [];
  for (const x of [a, b]) {
    const w = mod360(x.west);
    if (w + x.spanLon > 360) {
      parts.push([w, 360], [0, w + x.spanLon - 360]);
    } else parts.push([w, w + x.spanLon]);
  }
  parts.sort((p, q) => p[0] - q[0]);
  const merged: [number, number][] = [];
  for (const part of parts) {
    const last = merged[merged.length - 1];
    if (last && part[0] <= last[1]) last[1] = Math.max(last[1], part[1]);
    else merged.push([part[0], part[1]]);
  }
  // il vuoto piu' largo: fra un intervallo e il successivo, compreso quello che richiude il cerchio
  let bestGap = -1;
  let west = 0;
  for (let i = 0; i < merged.length; i++) {
    const cur = merged[i] as [number, number];
    const nxt = (merged[(i + 1) % merged.length] ?? cur) as [number, number];
    const gap = i === merged.length - 1 ? nxt[0] + 360 - cur[1] : nxt[0] - cur[1];
    if (gap > bestGap) {
      bestGap = gap;
      west = nxt[0];
    }
  }
  return { south, north, west: wrapLon(west), spanLon: Math.max(0, Math.min(360, 360 - bestGap)) };
}

/** Isole e pezzi staccati entro questa distanza (gradi) dal gruppo principale fanno parte del paese inquadrato; i territori lontani (Guyana francese, Hawaii... oltre) no. */
export const NEAR_DEG = 10;

/**
 * Il riquadro di una terra (calcolato una volta sola): quello del gruppo di poligoni vicini al piu' grande, cosi' la Francia si inquadra
 * senza la Guyana e le Figi, tagliate dall'antimeridiano, restano un riquadro piccolo. `null` se la geometria e' vuota.
 */
export function boxOf(f: Feature<Geometry>): GeoBox | null {
  const known = boxes.get(f);
  if (known !== undefined) return known;
  let box: GeoBox | null = null;
  try {
    const g = f.geometry;
    if (g && g.type === 'MultiPolygon' && g.coordinates.length > 1) {
      const polys = g.coordinates
        .map((coordinates) => {
          const poly: Geometry = { type: 'Polygon', coordinates };
          return { area: geoArea(poly), box: boxFromBounds(geoBounds(poly)) };
        })
        .filter((p): p is { area: number; box: GeoBox } => p.box !== null)
        .sort((x, y) => y.area - x.area)
        // i tagli lungo l'antimeridiano lasciano schegge larghe quanto il mondo e senza superficie (Figi): non contano
        .filter((p, i, all) => i === 0 || p.box.spanLon <= 90 || p.area >= (all[0]?.area ?? 0) * 0.01);
      const first = polys[0];
      if (first) {
        box = first.box;
        const rest = polys.slice(1);
        // il gruppo cresce finche' trova pezzi entro NEAR_DEG dal riquadro raggiunto
        for (let grew = true; grew; ) {
          grew = false;
          for (let i = rest.length - 1; i >= 0; i--) {
            const p = rest[i];
            if (p && boxGap(box, p.box) <= NEAR_DEG) {
              box = boxUnion(box, p.box);
              rest.splice(i, 1);
              grew = true;
            }
          }
        }
      }
    } else {
      box = boxFromBounds(geoBounds(f));
    }
  } catch {
    box = null;
  }
  boxes.set(f, box);
  return box;
}

const wholeBoxes = new WeakMap<object, GeoBox | null>();

/** Il riquadro di tutta la terra, territori lontani compresi: serve solo a scartare in fretta i punti fuori (per `countryAt`). */
function wholeBoxOf(f: Feature<Geometry>): GeoBox | null {
  const known = wholeBoxes.get(f);
  if (known !== undefined) return known;
  let box: GeoBox | null = null;
  try {
    box = boxFromBounds(geoBounds(f));
  } catch {
    box = null;
  }
  wholeBoxes.set(f, box);
  return box;
}

const englishName = (f: Feature<Geometry>): string => {
  const n = (f.properties as { name?: unknown } | null)?.name;
  return typeof n === 'string' ? n : '';
};

/** I paesi ordinati per nome nella lingua `lang`; le terre senza geometria si saltano. */
export function buildCountryList(fc: FeatureCollection, lang: string): CountryEntry[] {
  const out: CountryEntry[] = [];
  fc.features.forEach((f, index) => {
    const box = boxOf(f as Feature<Geometry>);
    if (!box) return;
    const name = countryName(f.id, englishName(f as Feature<Geometry>), lang);
    out.push({ index, name, box });
  });
  let collator: Intl.Collator | null = null;
  try {
    collator = new Intl.Collator(lang);
  } catch {
    collator = null;
  }
  out.sort((a, b) => (collator ? collator.compare(a.name, b.name) : a.name.localeCompare(b.name)));
  return out;
}

/** Il punto (latitudine, longitudine) dentro il riquadro (che puo' attraversare l'antimeridiano). */
export function boxContains(box: GeoBox, lat: number, lon: number): boolean {
  if (lat < box.south || lat > box.north) return false;
  if (box.spanLon >= 360) return true;
  return (((lon - box.west) % 360) + 360) % 360 <= box.spanLon;
}

/** Il paese in cui cade la stazione, o `null` (mare, posizione sconosciuta). */
export function countryAt(fc: FeatureCollection, lat: number, lon: number): number | null {
  if (!Number.isFinite(lat) || !Number.isFinite(lon)) return null;
  let found: number | null = null;
  fc.features.forEach((f, index) => {
    if (found !== null) return;
    const box = wholeBoxOf(f as Feature<Geometry>);
    if (!box || !boxContains(box, lat, lon)) return;
    if (geoContains(f, [lon, lat])) found = index;
  });
  return found;
}

/** La vista che inquadra il paese. */
export function countryView(box: GeoBox): MapView {
  return fitBox(box, COUNTRY_FIT);
}
