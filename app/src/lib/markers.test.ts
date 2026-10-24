import { describe, expect, it } from 'vitest';
import { hzAt, leftPercent } from './markers';

describe('markers', () => {
  it('places a frequency as a percentage of the audio band', () => {
    expect(leftPercent(0, 0, 3000)).toBe(0);
    expect(leftPercent(1500, 0, 3000)).toBe(50);
    expect(leftPercent(3000, 0, 3000)).toBe(100);
  });

  it('keeps markers inside the band', () => {
    expect(leftPercent(-50, 0, 3000)).toBe(0);
    expect(leftPercent(9000, 0, 3000)).toBe(100);
  });

  it('turns a position back into a whole number of hertz', () => {
    expect(hzAt(0.5, 0, 3000)).toBe(1500);
    expect(hzAt(0.3333, 0, 3000)).toBe(1000);
    expect(hzAt(-1, 0, 3000)).toBe(0);
    expect(hzAt(5, 0, 3000)).toBe(3000);
  });

  it('is the inverse of leftPercent', () => {
    for (const hz of [200, 958, 1500, 2999]) expect(hzAt(leftPercent(hz, 0, 3000) / 100, 0, 3000)).toBe(hz);
  });
});
