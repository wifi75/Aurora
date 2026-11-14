import { describe, expect, it } from 'vitest';
import { buildLogIndex, contestStatus, logStatus } from './logStatus';

const idx = buildLogIndex([
  { call: 'IU3ABC', band: '20m', mode: 'CW', country: 'Italy' },
  { call: 'DL1AA', band: '40m', mode: 'RTTY', country: 'Fed. Rep. of Germany' },
  { call: 'DL1AA', band: '20m', mode: 'CW', country: 'Fed. Rep. of Germany' },
]);

describe('logStatus', () => {
  it('gia collegato su banda e modo', () => {
    expect(logStatus(idx, 'iu3abc', 'Italy', '20m', 'cw')).toBe('worked');
  });
  it('banda nuova, modo nuovo, coppia nuova', () => {
    expect(logStatus(idx, 'IU3ABC', 'Italy', '40m', 'CW')).toBe('new_band');
    expect(logStatus(idx, 'IU3ABC', 'Italy', '20m', 'RTTY')).toBe('new_mode');
    expect(logStatus(idx, 'IU3ABC', 'Italy', '40m', 'RTTY')).toBe('new_band_mode');
    expect(logStatus(idx, 'DL1AA', null, '20m', 'RTTY')).toBe('new_band_mode');
  });
  it('paese nuovo e nominativo nuovo', () => {
    expect(logStatus(idx, 'JA1XYZ', 'Japan', '20m', 'CW')).toBe('mult');
    expect(logStatus(idx, 'DO1ID', 'Germany', '20m', 'CW')).toBe('new_call');
    expect(logStatus(idx, 'X1X', null, '20m', 'CW')).toBe('new_call');
    expect(logStatus(null, 'JA1XYZ', 'Japan', '20m', 'CW')).toBeNull();
  });
});

describe('contestStatus', () => {
  const marks = new Map([
    ['DL1AA', { call: 'DL1AA', doppio: true, molt_nuovi: [] }],
    ['JA1XYZ', { call: 'JA1XYZ', doppio: false, molt_nuovi: ['JA'] }],
    ['IU3ABC', { call: 'IU3ABC', doppio: false, molt_nuovi: [] }],
  ]);
  it('doppio e moltiplicatore dal contest', () => {
    expect(contestStatus(marks, idx, 'dl1aa', '40m', 'RTTY')).toBe('dupe');
    expect(contestStatus(marks, idx, 'JA1XYZ', '20m', 'CW')).toBe('mult');
  });
  it('novita rispetto al log del contest, niente segno sullo stesso slot non doppio', () => {
    expect(contestStatus(marks, idx, 'IU3ABC', '40m', 'CW')).toBe('new_band');
    expect(contestStatus(marks, idx, 'IU3ABC', '20m', 'CW')).toBeNull();
    expect(contestStatus(marks, idx, 'F1ABC', '20m', 'CW')).toBe('new_call');
    expect(contestStatus(null, null, 'DL1AA', '20m', 'CW')).toBeNull();
  });
});
