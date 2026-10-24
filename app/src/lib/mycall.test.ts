import { describe, expect, it } from 'vitest';
import { isCallToMe } from './mycall';

describe('isCallToMe', () => {
  it('is true when the first call of the message is mine', () => {
    expect(isCallToMe('AA1AA BB2BB -10', 'AA1AA')).toBe(true);
    expect(isCallToMe('AA1AA BB2BB KK00', 'AA1AA')).toBe(true);
    expect(isCallToMe('AA1AA BB2BB RR73', 'AA1AA')).toBe(true);
  });

  it('ignores case and surrounding spaces', () => {
    expect(isCallToMe('  aa1aa bb2bb -10 ', 'AA1AA')).toBe(true);
    expect(isCallToMe('AA1AA BB2BB -10', 'aa1aa')).toBe(true);
  });

  it('is false for my own transmissions, other stations and CQ calls', () => {
    expect(isCallToMe('BB2BB AA1AA -10', 'AA1AA')).toBe(false);
    expect(isCallToMe('CC3CC BB2BB -10', 'AA1AA')).toBe(false);
    expect(isCallToMe('CQ AA1AA FN42', 'AA1AA')).toBe(false);
  });

  it('is false without a known callsign, and never matches part of a call', () => {
    expect(isCallToMe('AA1AA BB2BB -10', '')).toBe(false);
    expect(isCallToMe('AA1AA BB2BB -10', undefined)).toBe(false);
    expect(isCallToMe('AA1AAX BB2BB -10', 'AA1AA')).toBe(false);
  });

  it('accepts a compound call addressed with its base call', () => {
    expect(isCallToMe('AA1AA/P BB2BB -10', 'AA1AA/P')).toBe(true);
  });
});
