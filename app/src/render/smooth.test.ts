import { describe, expect, it } from 'vitest';
import { RowAverager, RowClock } from './smooth';

describe('RowAverager', () => {
  it('returns the mean of every N rows and nothing in between', () => {
    const avg = new RowAverager(4);
    expect(avg.add(Uint8Array.from([0, 100]))).toBeNull();
    expect(avg.add(Uint8Array.from([40, 100]))).toBeNull();
    expect(avg.add(Uint8Array.from([80, 100]))).toBeNull();
    expect(Array.from(avg.add(Uint8Array.from([120, 100]))!)).toEqual([60, 100]);
  });

  it('starts a new group after each result', () => {
    const avg = new RowAverager(2);
    avg.add(Uint8Array.from([10]));
    expect(Array.from(avg.add(Uint8Array.from([30]))!)).toEqual([20]);
    avg.add(Uint8Array.from([200]));
    expect(Array.from(avg.add(Uint8Array.from([100]))!)).toEqual([150]);
  });

  it('restarts when the width changes and ignores empty rows', () => {
    const avg = new RowAverager(2);
    expect(avg.add(new Uint8Array(0))).toBeNull();
    avg.add(Uint8Array.from([10, 10]));
    expect(avg.add(Uint8Array.from([20]))).toBeNull();
    expect(Array.from(avg.add(Uint8Array.from([40]))!)).toEqual([30]);
  });

  it('refuses a group size below one', () => {
    expect(() => new RowAverager(0)).toThrow();
  });
});

describe('RowClock', () => {
  it('is 0 before the first row and grows to 1 between two rows', () => {
    const clock = new RowClock(200);
    expect(clock.fraction(1000)).toBe(0);
    clock.push(1000);
    expect(clock.fraction(1000)).toBe(0);
    expect(clock.fraction(1100)).toBeCloseTo(0.5, 5);
    expect(clock.fraction(5000)).toBe(1);
  });

  it('learns the interval from the arrivals and ignores huge gaps', () => {
    const clock = new RowClock(1000);
    let t = 1000;
    clock.push(t);
    for (let i = 0; i < 40; i++) {
      t += 250;
      clock.push(t);
    }
    expect(clock.fraction(t + 125)).toBeCloseTo(0.5, 1);
    clock.push(t + 60_000);
    expect(clock.fraction(t + 60_000 + 125)).toBeCloseTo(0.5, 1);
  });
});
