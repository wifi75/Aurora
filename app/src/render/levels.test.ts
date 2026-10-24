import { describe, expect, it } from 'vitest';
import { DEFAULT_LEVELS, clampLevel, readLevels, toneMap } from './levels';

describe('levels', () => {
  it('leaves the signal alone with the default levels', () => {
    for (const v of [0, 0.25, 0.5, 0.9, 1]) expect(toneMap(v, DEFAULT_LEVELS)).toBeCloseTo(v, 6);
  });

  it('black cuts the floor and rescales the rest', () => {
    const l = { ...DEFAULT_LEVELS, black: 0.4 };
    expect(toneMap(0.3, l)).toBe(0);
    expect(toneMap(1, l)).toBe(1);
    expect(toneMap(0.7, l)).toBeCloseTo(0.5, 6);
  });

  it('gain above one brightens the weak signals, below one darkens them', () => {
    expect(toneMap(0.25, { ...DEFAULT_LEVELS, gain: 2 })).toBeGreaterThan(0.25);
    expect(toneMap(0.25, { ...DEFAULT_LEVELS, gain: 0.5 })).toBeLessThan(0.25);
  });

  it('contrast pushes values away from the middle and stays inside 0-1', () => {
    const l = { ...DEFAULT_LEVELS, contrast: 2 };
    expect(toneMap(0.5, l)).toBeCloseTo(0.5, 6);
    expect(toneMap(0.6, l)).toBeCloseTo(0.7, 6);
    expect(toneMap(0.9, l)).toBe(1);
    expect(toneMap(0.1, l)).toBe(0);
  });

  it('reads the saved levels and ignores wrong ones', () => {
    const store: Record<string, string> = { 'aurora-wf-black': '0.2', 'aurora-wf-gain': 'abc', 'aurora-wf-contrast': '9' };
    expect(readLevels((k) => store[k] ?? null)).toEqual({ black: 0.2, gain: 1, contrast: 2.5 });
    expect(readLevels(() => null)).toEqual(DEFAULT_LEVELS);
    expect(clampLevel('black', -1)).toBe(0);
  });
});
