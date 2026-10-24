import { describe, expect, it } from 'vitest';
import type { ClusterSpot } from '../api/client';
import { filterSpots, spotAge, spotsOnWaterfall, toList } from './cluster';

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

describe('spotsOnWaterfall', () => {
  const NOW = 2_000_000;
  const at = (id: number, call: string, khz: number, over: Partial<ClusterSpot> = {}): ClusterSpot => ({ ...spot(id, call, '20m', 'FT8'), freq_khz: khz, received_ms: NOW - 60_000, ...over });
  const DIAL = 14_074_000;

  it('puts a spot at its offset from the radio frequency', () => {
    const [s] = spotsOnWaterfall([at(1, 'JA1ABC', 14074.5)], DIAL, 0, 3000, NOW);
    expect(s).toMatchObject({ call: 'JA1ABC', hz: 500, row: 0 });
  });

  it('leaves out spots outside the waterfall, with no dial frequency, or too old', () => {
    const list = [at(1, 'LOW1A', 14073.9), at(2, 'HIGH1A', 14077.5), at(3, 'OLD1A', 14075, { received_ms: NOW - 700_000 }), at(4, 'OK1A', 14075)];
    expect(spotsOnWaterfall(list, DIAL, 0, 3000, NOW).map((s) => s.call)).toEqual(['OK1A']);
    expect(spotsOnWaterfall(list, 0, 0, 3000, NOW)).toEqual([]);
  });

  it('keeps only the latest spot of each callsign', () => {
    const list = [at(1, 'K1ABC', 14075, { received_ms: NOW - 120_000 }), at(2, 'k1abc', 14076, { received_ms: NOW - 30_000 })];
    const result = spotsOnWaterfall(list, DIAL, 0, 3000, NOW);
    expect(result).toHaveLength(1);
    expect(result[0]?.hz).toBe(2000);
  });

  it('stacks labels that would overlap on other rows, and drops the label when there is no row left', () => {
    const near = [at(1, 'AA1AA', 14075.0), at(2, 'BB1BB', 14075.05), at(3, 'CC1CC', 14075.1), at(4, 'DD1DD', 14075.15)];
    const result = spotsOnWaterfall(near, DIAL, 0, 3000, NOW);
    expect(result.map((s) => s.row)).toEqual([0, 1, 2, null]);
    const apart = spotsOnWaterfall([at(1, 'AA1AA', 14075.0), at(2, 'BB1BB', 14076.0)], DIAL, 0, 3000, NOW);
    expect(apart.map((s) => s.row)).toEqual([0, 0]);
  });
});
