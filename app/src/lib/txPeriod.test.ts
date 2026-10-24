import { describe, expect, it } from 'vitest';
import { periodLabel } from './txPeriod';

describe('periodLabel', () => {
  it('uses the text the engine writes, as Tx ..., whatever the mode', () => {
    expect(periodLabel(true, 'FT8', 'TX 00/30')).toBe('Tx 00/30');
    expect(periodLabel(false, 'FT8', 'TX 15/45')).toBe('Tx 15/45');
    expect(periodLabel(true, 'FT4', 'TX 00')).toBe('Tx 00');
    expect(periodLabel(false, 'JT', ' TX 15 ')).toBe('Tx 15');
  });

  it('rebuilds the FT8 label while the engine text has not been read yet, and has none for the other modes', () => {
    expect(periodLabel(true, 'FT8')).toBe('Tx 00/30');
    expect(periodLabel(false, 'FT8')).toBe('Tx 15/45');
    expect(periodLabel(true, 'FT4')).toBeNull();
    expect(periodLabel(false, undefined)).toBeNull();
  });
});
