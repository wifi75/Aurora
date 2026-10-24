import { describe, expect, it } from 'vitest';
import { rowsPerStep } from './Waterfall';

describe('rowsPerStep', () => {
  it('defaults to one row per step: no averaging, the waterfall runs at the core rate', () => {
    expect(rowsPerStep(null)).toBe(1);
    expect(rowsPerStep(undefined)).toBe(1);
    expect(rowsPerStep('')).toBe(1);
  });

  it('accepts 1 to 4 and ignores anything else', () => {
    expect(rowsPerStep('2')).toBe(2);
    expect(rowsPerStep('4')).toBe(4);
    expect(rowsPerStep('0')).toBe(1);
    expect(rowsPerStep('5')).toBe(1);
    expect(rowsPerStep('2.5')).toBe(1);
    expect(rowsPerStep('abc')).toBe(1);
  });
});
