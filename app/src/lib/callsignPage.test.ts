import { describe, expect, it } from 'vitest';
import { cleanCall, qrzUrl } from './callsignPage';

describe('callsignPage', () => {
  it('builds the QRZ.com address with the callsign encoded', () => {
    expect(qrzUrl('iu3abc')).toBe('https://www.qrz.com/db/IU3ABC');
    expect(qrzUrl('EA8/IU3ABC')).toBe('https://www.qrz.com/db/EA8%2FIU3ABC');
    expect(qrzUrl('<K1ABC>')).toBe('https://www.qrz.com/db/K1ABC');
  });

  it('refuses words that are not callsigns', () => {
    expect(qrzUrl('')).toBeNull();
    expect(qrzUrl('K1ABC?x=1')).toBeNull();
    expect(cleanCall('CQ')).toBeNull();
  });
});
