import { feature } from 'topojson-client';
import type { FeatureCollection } from 'geojson';
import type { GeometryCollection, Topology } from 'topojson-specification';
import topo from 'world-atlas/countries-50m.json';
import { describe, expect, it } from 'vitest';
import { alpha2Of, countryName } from './countryCodes';
import { boxContains, boxFromBounds, boxGap, boxUnion, buildCountryList, countryAt, countryView } from './satCountries';
import { ZOOM_MAX, projectView } from './satView';

const t = topo as unknown as Topology;
const fc = feature(t, t.objects.countries as GeometryCollection) as FeatureCollection;
const nameOf = (f: FeatureCollection['features'][number]) => (f.properties as { name: string }).name;

describe('nomi dei paesi', () => {
  it('ogni terra di Natural Earth ha un nome non vuoto in ogni lingua dell’interfaccia', () => {
    for (const lang of ['it', 'en', 'de', 'fr', 'es', 'pt', 'nl']) {
      for (const f of fc.features) {
        const n = countryName(f.id, nameOf(f), lang);
        expect(n.trim(), `${nameOf(f)} (${lang})`).not.toBe('');
      }
    }
  });

  it('i codici numerici si risolvono in alpha-2; restano senza solo le terre senza codice proprio', () => {
    const senza = fc.features.filter((f) => alpha2Of(f.id, nameOf(f)) === undefined).map(nameOf);
    expect(senza.sort()).toEqual(['Ashmore and Cartier Is.', 'Indian Ocean Ter.', 'N. Cyprus', 'Siachen Glacier', 'Somaliland']);
    expect(alpha2Of('380', 'Italy')).toBe('IT');
    expect(alpha2Of('016', 'American Samoa')).toBe('AS');
    expect(alpha2Of(undefined, 'Kosovo')).toBe('XK');
  });

  it('nella lingua scelta; in mancanza il nome inglese per esteso', () => {
    expect(countryName('380', 'Italy', 'it')).toBe('Italia');
    expect(countryName('276', 'Germany', 'de')).toBe('Deutschland');
    expect(countryName('380', 'Italy', 'en')).toBe('Italy');
    expect(countryName(undefined, 'N. Cyprus', 'it')).toBe('Northern Cyprus');
    expect(countryName('036', 'Ashmore and Cartier Is.', 'it')).toBe('Ashmore and Cartier Islands');
  });

  it('l’elenco e’ ordinato per nome nella lingua e senza doppioni', () => {
    const it = buildCountryList(fc, 'it');
    expect(it.length).toBe(fc.features.length);
    const names = it.map((c) => c.name);
    expect(new Set(names).size).toBe(names.length);
    const sorted = [...names].sort(new Intl.Collator('it').compare);
    expect(names).toEqual(sorted);
    expect(names).toContain('Italia');
    expect(buildCountryList(fc, 'en').map((c) => c.name)).toContain('Italy');
  });
});

describe('riquadri dei paesi', () => {
  const list = buildCountryList(fc, 'en');
  const box = (name: string) => {
    const c = list.find((x) => x.name === name);
    if (!c) throw new Error(`manca ${name}`);
    return c.box;
  };

  it('da geoBounds di d3 al riquadro, con o senza antimeridiano', () => {
    expect(boxFromBounds([[10, 40], [20, 50]])).toEqual({ south: 40, north: 50, west: 10, spanLon: 10 });
    expect(boxFromBounds([[170, -20], [-170, -10]])).toEqual({ south: -20, north: -10, west: 170, spanLon: 20 });
    expect(boxFromBounds([[Number.NaN, 0], [1, 1]])).toBeNull();
  });

  it('la Russia attraversa l’antimeridiano: contiene le due sponde e non l’Atlantico', () => {
    const r = box('Russia');
    expect(r.spanLon).toBeLessThan(200);
    expect(boxContains(r, 65, 179)).toBe(true);
    expect(boxContains(r, 65, -175)).toBe(true);
    expect(boxContains(r, 55, 37)).toBe(true);
    expect(boxContains(r, 55, -40)).toBe(false);
    const v = countryView(r);
    for (const [lat, lon] of [[55, 37], [65, 179], [65, -175]] as const) {
      const [x] = projectView(lat, lon, 900, 450, v);
      expect(x).toBeGreaterThanOrEqual(-1);
      expect(x).toBeLessThanOrEqual(901);
    }
  });

  it('Figi e Stati Uniti: riquadri a cavallo che prendono il lato corto', () => {
    const fiji = box('Fiji');
    expect(fiji.spanLon).toBeLessThan(10);
    expect(boxContains(fiji, -17.5, 178)).toBe(true);
    expect(boxContains(fiji, -17.5, -179.9)).toBe(true);
    const us = box('United States');
    expect(us.spanLon).toBeLessThan(200);
    expect(boxContains(us, 40, -100)).toBe(true);
    expect(boxContains(us, 64, -150)).toBe(true);
  });

  it('i territori lontani non allargano il riquadro: la Francia e’ quella europea, non la Guyana', () => {
    const fr = box('France');
    expect(boxContains(fr, 48.8, 2.3)).toBe(true);
    expect(boxContains(fr, 4, -53)).toBe(false);
    expect(fr.spanLon).toBeLessThan(25);
  });

  it('unione e distanza di riquadri sul cerchio', () => {
    const a = { south: 0, north: 10, west: 170, spanLon: 8 };
    const b = { south: 5, north: 20, west: -175, spanLon: 4 };
    const u = boxUnion(a, b);
    expect(u).toEqual({ south: 0, north: 20, west: 170, spanLon: 19 });
    expect(boxGap(a, b)).toBe(7);
    expect(boxGap(a, { south: 0, north: 10, west: 175, spanLon: 2 })).toBe(0);
    expect(boxGap(a, { south: 30, north: 40, west: 172, spanLon: 2 })).toBe(20);
    expect(boxUnion(a, { south: 0, north: 1, west: -180, spanLon: 360 }).spanLon).toBe(360);
  });

  it('un’isola minuscola non e’ ingrandita oltre lo zoom massimo; l’Antartide resta intera', () => {
    expect(countryView(box('Malta')).zoom).toBe(ZOOM_MAX);
    expect(countryView(box('Antarctica')).zoom).toBe(1);
    const it = countryView(box('Italy'));
    expect(it.centerLat).toBeGreaterThan(38);
    expect(it.centerLat).toBeLessThan(46);
    expect(it.centerLon).toBeGreaterThan(8);
    expect(it.centerLon).toBeLessThan(16);
  });

  it('il paese della stazione: dentro il paese, null in mare', () => {
    const at = (lat: number, lon: number) => {
      const i = countryAt(fc, lat, lon);
      return i === null ? null : nameOf(fc.features[i] as FeatureCollection['features'][number]);
    };
    expect(at(45.4, 11.9)).toBe('Italy');
    expect(at(48.85, 2.35)).toBe('France');
    expect(at(-17.8, 178)).toBe('Fiji');
    expect(at(4, -53)).toBe('France');
    expect(at(0, -30)).toBeNull();
    expect(countryAt(fc, Number.NaN, 0)).toBeNull();
  });
});
