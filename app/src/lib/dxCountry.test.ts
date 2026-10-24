import { describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { countryOfCall } from './dxCountry';

const decode = (id: number, text: string, country: string | null): DecodedMessage => ({ id, time_ms: 0, snr: 0, delta_time: 0, freq_hz: 1000, text, tag: null, country });

describe('countryOfCall', () => {
  const list = [decode(3, 'IU3ABC DO1ID -10', 'Fed. Rep. of Germany'), decode(2, 'CQ K1ABC FN42', 'United States'), decode(1, 'CQ DO1ID JO41', 'Fed. Rep. of Germany')];

  it('finds the country of the station that sent the message', () => {
    expect(countryOfCall(list, 'DO1ID')).toBe('Fed. Rep. of Germany');
    expect(countryOfCall(list, 'k1abc')).toBe('United States');
  });

  it('is null for an unknown or empty call', () => {
    expect(countryOfCall(list, 'ZZ9ZZ')).toBeNull();
    expect(countryOfCall(list, '')).toBeNull();
    expect(countryOfCall(list, undefined)).toBeNull();
  });
});
