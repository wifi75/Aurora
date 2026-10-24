import { describe, expect, it } from 'vitest';
import type { PskReport } from '../api/client';
import { bandCounts, bestSnr, conditionGroup, latestPerReceiver, pskRows, summarize } from './psk';

const report = (receiver: string, locator: string, freq_hz: number, over: Partial<PskReport> = {}): PskReport => ({ receiver, locator, snr: -10, freq_hz, mode: 'FT8', time_s: 100, country: null, ...over });

describe('pskRows', () => {
  it('adds the band and the distance from my grid', () => {
    const [near, far] = pskRows([report('G0XYZ', 'IO91', 14_075_000), report('JA1ABC', 'PM95', 7_075_000)], 'IO91wm');
    expect(near?.band).toBe('20m');
    expect(near?.km).toBeLessThan(100);
    expect(far?.band).toBe('40m');
    expect(far?.km).toBeGreaterThan(9000);
  });

  it('uses the first six characters of a longer locator', () => {
    const [row] = pskRows([report('G0XYZ', 'IO91WM12', 14_075_000)], 'IO91wm');
    expect(row?.km).not.toBeNull();
  });

  it('leaves the distance empty when a grid is missing or wrong', () => {
    expect(pskRows([report('X1X', '', 14_075_000), report('Y1Y', 'ZZ99', 14_075_000)], 'IO91wm').map((r) => r.km)).toEqual([null, null]);
    expect(pskRows([report('X1X', 'IO91', 14_075_000)], null)[0]?.km).toBeNull();
    expect(pskRows([report('X1X', 'IO91', 3_000_000)], 'IO91wm')[0]?.band).toBeNull();
  });
});

describe('summarize', () => {
  it('counts the different stations and finds the farthest', () => {
    const rows = pskRows([report('G0XYZ', 'IO91', 14_075_000), report('G0XYZ', 'IO91', 7_075_000), report('JA1ABC', 'PM95', 14_075_000)], 'IO91wm');
    const s = summarize(rows);
    expect(s.stations).toBe(2);
    expect(s.farthest?.call).toBe('JA1ABC');
  });

  it('has no farthest without distances', () => {
    expect(summarize(pskRows([report('G0XYZ', '', 14_075_000)], 'IO91wm')).farthest).toBeNull();
    expect(summarize([])).toEqual({ stations: 0, farthest: null });
  });
});

describe('conditionGroup', () => {
  it('maps a band to the group the forecast uses', () => {
    expect(conditionGroup('40m')).toBe('80m-40m');
    expect(conditionGroup('20m')).toBe('30m-20m');
    expect(conditionGroup('15m')).toBe('17m-15m');
    expect(conditionGroup('10m')).toBe('12m-10m');
    expect(conditionGroup('160m')).toBeNull();
    expect(conditionGroup(null)).toBeNull();
  });
});

describe('band counts, best report and one line per receiver', () => {
  const rows = pskRows(
    [
      report('G0XYZ', 'IO91', 14_075_000, { snr: -5, time_s: 10 }),
      report('G0XYZ', 'IO91', 14_075_000, { snr: 2, time_s: 20 }),
      report('G0XYZ', 'IO91', 7_075_000, { snr: -9, time_s: 15 }),
      report('JA1ABC', 'PM95', 14_075_000, { snr: -20, time_s: 5 }),
      report('K1ABC', 'FN42', 3_000_000, { snr: null, time_s: 1 }),
    ],
    'IO91wm',
  );

  it('counts the different stations per band in band order and skips unknown bands', () => {
    expect(bandCounts(rows)).toEqual([{ band: '40m', count: 1 }, { band: '20m', count: 2 }]);
    expect(bandCounts([])).toEqual([]);
  });

  it('finds the best report', () => {
    expect(bestSnr(rows)).toEqual({ call: 'G0XYZ', snr: 2 });
    expect(bestSnr(pskRows([report('X1X', 'IO91', 14_075_000, { snr: null })], null))).toBeNull();
  });

  it('keeps the latest report of each receiver on each band', () => {
    const latest = latestPerReceiver(rows);
    expect(latest).toHaveLength(4);
    expect(latest.find((r) => r.receiver === 'G0XYZ' && r.band === '20m')?.snr).toBe(2);
  });
});
