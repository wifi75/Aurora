import { describe, expect, it } from 'vitest';
import type { ClusterSpot } from '../api/client';
import { filterSpots, spotAge } from './cluster';

const spot = (id: number, call: string, band: string | null, mode: string | null): ClusterSpot => ({
  id, received_ms: 1_000_000, server: 'Prova', spotter: 'W3LPL', freq_khz: 14074, dx_call: call, comment: '', time_utc: '1234', grid: null, band, mode, country: null, continent: null,
});

describe('filterSpots', () => {
  const list = [spot(4, 'JA1ABC', '20m', 'FT8'), spot(3, 'K1ABC', '40m', 'CW'), spot(2, 'DL1XYZ', '20m', 'SSB'), spot(1, 'ZZ9ZZ', null, null)];

  it('keeps everything with the default filters', () => {
    expect(filterSpots(list, { band: 'all', mode: 'all', search: '' }, '20m')).toHaveLength(4);
  });

  it('shows only the band JTDX is on', () => {
    expect(filterSpots(list, { band: 'now', mode: 'all', search: '' }, '20m').map((s) => s.id)).toEqual([4, 2]);
    expect(filterSpots(list, { band: 'now', mode: 'all', search: '' }, null)).toEqual([list[3]]);
  });

  it('filters by mode and by part of the callsign, ignoring case', () => {
    expect(filterSpots(list, { band: 'all', mode: 'CW', search: '' }, null).map((s) => s.dx_call)).toEqual(['K1ABC']);
    expect(filterSpots(list, { band: 'all', mode: 'all', search: ' abc ' }, null).map((s) => s.dx_call)).toEqual(['JA1ABC', 'K1ABC']);
  });

  it('keeps the order and combines the filters', () => {
    expect(filterSpots(list, { band: 'now', mode: 'FT8', search: 'ja' }, '20m').map((s) => s.id)).toEqual([4]);
  });
});

describe('spotAge', () => {
  it('counts the seconds since the spot arrived and never goes negative', () => {
    expect(spotAge(spot(1, 'K1ABC', null, null), 1_090_000)).toBe(90);
    expect(spotAge(spot(1, 'K1ABC', null, null), 900_000)).toBe(0);
  });
});
