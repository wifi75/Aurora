import { describe, expect, it } from 'vitest';
import { contestInGroup } from './ModePages';

describe('contest branch of each mode', () => {
  it('puts a contest under FT8 · FT4 and/or VHF · EME by its modes', () => {
    expect(contestInGroup(['FT8', 'FT4'], 'ft8')).toBe(true);
    expect(contestInGroup(['FT8', 'FT4'], 'vhf')).toBe(false);
    expect(contestInGroup(['FT8', 'FT4', 'MSK144', 'Q65'], 'vhf')).toBe(true);
    expect(contestInGroup(['FT8', 'FT4', 'MSK144', 'Q65'], 'ft8')).toBe(true);
    expect(contestInGroup(['q65'], 'vhf')).toBe(true);
  });

  it('keeps a contest without its modes (older engine) under FT8 · FT4', () => {
    expect(contestInGroup(undefined, 'ft8')).toBe(true);
    expect(contestInGroup([], 'vhf')).toBe(false);
  });
});
