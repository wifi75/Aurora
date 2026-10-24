import { describe, expect, it } from 'vitest';
import type { ClusterSpot } from '../api/client';
import { filterSpots, spotAge, toList } from './cluster';

const spot = (id: number, call: string, band: string | null, mode: string | null): ClusterSpot => ({
  id, received_ms: 1_000_000, server: 'Prova', spotter: 'W3LPL', freq_khz: 14074, dx_call: call, comment: '', time_utc: '1234', grid: null, band, mode, country: null, continent: null,
});

describe('filterSpots', () => {
  const list = [spot(4, 'JA1ABC', '20m', 'FT8'), spot(3, 'K1ABC', '40m', 'CW'), spot(2, 'DL1XYZ', '20m', 'SSB'), spot(1, 'ZZ9ZZ', null, null)];

  it('keeps everything with the default filters', () => {
    expect(filterSpots(list, { bands: [], modes: [], search: '' }, '20m')).toHaveLength(4);
  });

  it('shows only the band the engine is on', () => {
    expect(filterSpots(list, { bands: ['now'], modes: [], search: '' }, '20m').map((s) => s.id)).toEqual([4, 2]);
    expect(filterSpots(list, { bands: ['now'], modes: [], search: '' }, null)).toEqual([list[3]]);
  });

  it('filters by mode and by part of the callsign, ignoring case', () => {
    expect(filterSpots(list, { bands: [], modes: ['CW'], search: '' }, null).map((s) => s.dx_call)).toEqual(['K1ABC']);
    expect(filterSpots(list, { bands: [], modes: [], search: ' abc ' }, null).map((s) => s.dx_call)).toEqual(['JA1ABC', 'K1ABC']);
  });

  it('accepts several bands and several modes at once', () => {
    expect(filterSpots(list, { bands: ['20m', '40m'], modes: [], search: '' }, null).map((s) => s.id)).toEqual([4, 3, 2]);
    expect(filterSpots(list, { bands: [], modes: ['FT8', 'SSB'], search: '' }, null).map((s) => s.id)).toEqual([4, 2]);
    expect(filterSpots(list, { bands: ['now', '40m'], modes: [], search: '' }, '20m').map((s) => s.id)).toEqual([4, 3, 2]);
  });

  it('turns the single value of the old versions into a list', () => {
    expect(toList('all')).toEqual([]);
    expect(toList('now')).toEqual(['now']);
    expect(toList(['20m', 3])).toEqual(['20m']);
    expect(toList(undefined)).toEqual([]);
  });

  it('keeps the order and combines the filters', () => {
    expect(filterSpots(list, { bands: ['now'], modes: ['FT8'], search: 'ja' }, '20m').map((s) => s.id)).toEqual([4]);
  });
});

describe('spotAge', () => {
  it('counts the seconds since the spot arrived and never goes negative', () => {
    expect(spotAge(spot(1, 'K1ABC', null, null), 1_090_000)).toBe(90);
    expect(spotAge(spot(1, 'K1ABC', null, null), 900_000)).toBe(0);
  });
});
