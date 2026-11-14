import { describe, expect, it } from 'vitest';
import { buildLogIndex, logStatus } from './logStatus';

const idx = buildLogIndex([
  { call: 'IU3ABC', band: '20m', mode: 'CW', country: 'Italy' },
  { call: 'DL1AA', band: '40m', mode: 'RTTY', country: 'Fed. Rep. of Germany' },
]);

describe('logStatus', () => {
  it('riconosce gia collegato su banda e modo', () => {
    expect(logStatus(idx, 'iu3abc', 'Italy', '20m', 'cw')).toBe('worked');
  });
  it('collegato altrove', () => {
    expect(logStatus(idx, 'IU3ABC', 'Italy', '40m', 'CW')).toBe('worked_other');
    expect(logStatus(idx, 'DL1AA', null, '40m', 'CW')).toBe('worked_other');
  });
  it('paese nuovo e paese gia collegato', () => {
    expect(logStatus(idx, 'JA1XYZ', 'Japan', '20m', 'CW')).toBe('new_dxcc');
    expect(logStatus(idx, 'DO1ID', 'Germany', '20m', 'CW')).toBeNull();
    expect(logStatus(idx, 'X1X', null, '20m', 'CW')).toBeNull();
    expect(logStatus(null, 'JA1XYZ', 'Japan', '20m', 'CW')).toBeNull();
  });
});
