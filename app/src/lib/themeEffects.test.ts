import { describe, expect, it } from 'vitest';
import { DEFAULT_THEME_EFFECTS, fpsOk, fxVisible, hardwarePowerful, inWinterPeriod, readThemeEffects } from './themeEffects';

describe('inWinterPeriod', () => {
  it('starts on 1 December and ends on 6 January, both included', () => {
    expect(inWinterPeriod(new Date(2026, 10, 30, 23, 59))).toBe(false);
    expect(inWinterPeriod(new Date(2026, 11, 1, 0, 0))).toBe(true);
    expect(inWinterPeriod(new Date(2026, 11, 31, 12, 0))).toBe(true);
    expect(inWinterPeriod(new Date(2027, 0, 1, 0, 0))).toBe(true);
    expect(inWinterPeriod(new Date(2027, 0, 6, 23, 59))).toBe(true);
    expect(inWinterPeriod(new Date(2027, 0, 7, 0, 0))).toBe(false);
    expect(inWinterPeriod(new Date(2027, 6, 15))).toBe(false);
  });
});

describe('fxVisible', () => {
  const winter = new Date(2026, 11, 10);
  const summer = new Date(2026, 6, 10);
  it('on always shows, off never shows', () => {
    expect(fxVisible('on', summer, 'slow')).toBe(true);
    expect(fxVisible('off', winter, 'fast')).toBe(false);
  });
  it('auto shows only in the period on a fast PC', () => {
    expect(fxVisible('auto', winter, 'fast')).toBe(true);
    expect(fxVisible('auto', winter, 'slow')).toBe(false);
    expect(fxVisible('auto', winter, 'unknown')).toBe(false);
    expect(fxVisible('auto', summer, 'fast')).toBe(false);
  });
});

describe('power', () => {
  it('needs 6 cores and 4 GB when the memory is known', () => {
    expect(hardwarePowerful(8, 8)).toBe(true);
    expect(hardwarePowerful(6, undefined)).toBe(true);
    expect(hardwarePowerful(4, 8)).toBe(false);
    expect(hardwarePowerful(8, 2)).toBe(false);
    expect(hardwarePowerful(undefined, 8)).toBe(false);
  });
  it('fps average at least 45', () => {
    expect(fpsOk(120, 2000)).toBe(true);
    expect(fpsOk(80, 2000)).toBe(false);
    expect(fpsOk(10, 0)).toBe(false);
  });
});

describe('readThemeEffects', () => {
  it('defaults: snow auto, lights off; bad values fall back', () => {
    expect(readThemeEffects(null)).toEqual(DEFAULT_THEME_EFFECTS);
    expect(DEFAULT_THEME_EFFECTS).toEqual({ snow: 'auto', lights: 'off' });
    const s = { getItem: () => JSON.stringify({ snow: 'off', lights: 'boh' }) };
    expect(readThemeEffects(s)).toEqual({ snow: 'off', lights: 'off' });
  });
});
