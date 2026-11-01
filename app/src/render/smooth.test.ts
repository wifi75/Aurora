import { describe, expect, it } from 'vitest';
import { RowClock } from './smooth';

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
