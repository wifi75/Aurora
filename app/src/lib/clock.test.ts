import { describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { clockInTolerance, medianDt, offsetLabel } from './clock';

const d = (id: number, timeMs: number, dt: number): DecodedMessage => ({ id, time_ms: timeMs, snr: -10, delta_time: dt, freq_hz: 1000, text: 'CQ AA1AA FN42', tag: null });

describe('clock helpers', () => {
  it('tells behind from ahead', () => {
    expect(offsetLabel(120.4)).toEqual({ ms: 120, behind: true });
    expect(offsetLabel(-80.6)).toEqual({ ms: 81, behind: false });
  });

  it('uses the threshold on the absolute error', () => {
    expect(clockInTolerance(150, 200)).toBe(true);
    expect(clockInTolerance(-250, 200)).toBe(false);
    expect(clockInTolerance(200, 200)).toBe(true);
  });

  it('takes the median DT of the last period, like the Med of JTDX', () => {
    const list = [d(5, 100_000, 0.3), d(4, 100_000, 0.1), d(3, 99_000, 0.2), d(2, 98_000, 2.5), d(1, 10_000, 9)];
    expect(medianDt(list)).toBe(0.25);
  });

  it('needs at least three recent messages and ignores our own transmissions', () => {
    expect(medianDt([])).toBeNull();
    expect(medianDt([d(2, 100_000, 0.1), d(1, 100_000, 0.2)])).toBeNull();
    expect(medianDt([d(3, 100_000, 0.1), d(-1, 100_000, 5), d(2, 100_000, 0.2), d(1, 100_000, 0.3)])).toBe(0.2);
  });
});
