import { describe, expect, it } from 'vitest';
import { filledFrom, isCallsign } from './callLookup';

describe('ricerca del nominativo preso dalla ricezione', () => {
  it('riconosce i nominativi e scarta le parole', () => {
    expect(isCallsign('IU3ABC')).toBe(true);
    expect(isCallsign('iu3abc/p')).toBe(true);
    expect(isCallsign('599')).toBe(false);
    expect(isCallsign('NAME')).toBe(false);
    expect(isCallsign('5NN')).toBe(true);
  });

  it('prende nome, citta, locatore e paese dalla risposta', () => {
    const got = filledFrom({
      call: 'IU3ABC',
      dxcc: { country: 'Italy' } as never,
      person: { fname: 'Joe', name: 'Joseph', city: 'Milano', country: 'Italy', grid: 'io91wm', source: 'qrz' } as never,
      lookup: 'found',
      subscriber: null,
      message: null,
    });
    expect(got).toEqual({ name: 'Joe', qth: 'Milano', wwloc: 'IO91WM', country: 'Italy' });
  });
});
