import { describe, expect, it } from 'vitest';
import { EMPTY_WISHLIST, baseCall, parseLines, senderOf, wishReason, type Wishlist } from './wishlist';

const list: Wishlist = { ...EMPTY_WISHLIST, enabled: true, calls: ['K1ABC', '3Y0J'], prefixes: ['VP8'], countries: ['Bulgaria'], grids: ['JN45'] };

describe('wishReason', () => {
  it('matches the call (with suffix or prefix), then the prefix, the country and the grid', () => {
    expect(wishReason(list, 'K1ABC')).toBe('call');
    expect(wishReason(list, 'k1abc/p')).toBe('call');
    expect(wishReason(list, 'F/K1ABC')).toBe('call');
    expect(wishReason(list, 'VP8LP')).toBe('prefix');
    expect(wishReason(list, 'LZ1AB', null, 'bulgaria')).toBe('country');
    expect(wishReason(list, 'IU1XYZ', 'JN45ab')).toBe('grid');
    expect(wishReason(list, 'K1ABC', 'JN45', 'Bulgaria')).toBe('call');
    expect(wishReason(list, 'W1AW', 'FN31', 'United States')).toBeNull();
  });

  it('does not match part of another call, and nothing when it is off', () => {
    expect(wishReason(list, 'K1ABCD')).toBeNull();
    expect(wishReason(list, 'AK1ABC')).toBeNull();
    expect(wishReason({ ...list, enabled: false }, 'K1ABC')).toBeNull();
  });

  it('the band filter only excludes known bands', () => {
    const only20 = { ...list, bands: ['20m'] };
    expect(wishReason(only20, 'K1ABC', null, null, '20m')).toBe('call');
    expect(wishReason(only20, 'K1ABC', null, null, '40m')).toBeNull();
    expect(wishReason(only20, 'K1ABC', null, null, null)).toBe('call');
  });
});

describe('senderOf', () => {
  it('reads who transmits and the locator', () => {
    expect(senderOf('CQ K1ABC FN42')).toEqual({ call: 'K1ABC', grid: 'FN42' });
    expect(senderOf('CQ DX K1ABC FN42')).toEqual({ call: 'K1ABC', grid: 'FN42' });
    expect(senderOf('K1ABC G0XYZ IO91')).toEqual({ call: 'G0XYZ', grid: 'IO91' });
    expect(senderOf('CQ K1ABC')).toEqual({ call: 'K1ABC', grid: null });
    expect(senderOf('K1ABC G0XYZ -12')).toEqual({ call: 'G0XYZ', grid: null });
    expect(senderOf('K1ABC G0XYZ RR73')).toEqual({ call: 'G0XYZ', grid: null });
    expect(senderOf('TNX 73')).toBeNull();
    expect(senderOf('CQ')).toBeNull();
  });
});

describe('helpers', () => {
  it('finds the base call', () => {
    expect(baseCall('K1ABC/P')).toBe('K1ABC');
    expect(baseCall('F/K1ABC')).toBe('K1ABC');
    expect(baseCall('K1ABC')).toBe('K1ABC');
  });

  it('splits a list by lines, commas and semicolons', () => {
    expect(parseLines('K1ABC\n 3Y0J ,VP8;\n\n')).toEqual(['K1ABC', '3Y0J', 'VP8']);
    expect(parseLines('')).toEqual([]);
  });
});
