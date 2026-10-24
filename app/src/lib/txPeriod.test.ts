import { describe, expect, it } from 'vitest';
import { periodLabel } from './txPeriod';

describe('periodLabel', () => {
  it('says the seconds of the period in FT8: on is even (00/30), off is odd (15/45)', () => {
    expect(periodLabel(true, 'FT8')).toBe('Tx 00/30');
    expect(periodLabel(false, 'FT8')).toBe('Tx 15/45');
  });

  it('has its own labels in FT4 and none for the other modes', () => {
    expect(periodLabel(true, 'FT4')).toBe('Tx 00/15');
    expect(periodLabel(false, 'FT4')).toBe('Tx 7,5/22,5');
    expect(periodLabel(true, 'JT')).toBeNull();
    expect(periodLabel(false, undefined)).toBeNull();
  });
});
