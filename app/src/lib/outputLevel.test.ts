import { describe, expect, it } from 'vitest';
import { attenuationFromPercent, levelPercent } from './outputLevel';

describe('output level', () => {
  it('maps the attenuation of the engine (0-45 dB) to a level in percent, 100 = no attenuation', () => {
    expect(levelPercent(0)).toBe(100);
    expect(levelPercent(45)).toBe(0);
    expect(levelPercent(22.5)).toBe(50);
    expect(levelPercent(39.2)).toBe(13);
  });

  it('is clamped', () => {
    expect(levelPercent(-5)).toBe(100);
    expect(levelPercent(99)).toBe(0);
  });

  it('maps a level back to an attenuation in tenths of a dB', () => {
    expect(attenuationFromPercent(100)).toBe(0);
    expect(attenuationFromPercent(0)).toBe(45);
    expect(attenuationFromPercent(50)).toBe(22.5);
    expect(attenuationFromPercent(13)).toBe(39.2);
  });

  it('is clamped too', () => {
    expect(attenuationFromPercent(150)).toBe(0);
    expect(attenuationFromPercent(-10)).toBe(45);
  });
});
