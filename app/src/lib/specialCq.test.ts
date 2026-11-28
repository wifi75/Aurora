import { describe, expect, it } from 'vitest';
import { baseCall, parseSpecialCq } from './specialCq';

describe('parseSpecialCq', () => {
  it('riconosce i cinque CQ speciali, con e senza locatore', () => {
    expect(parseSpecialCq('CQ POTA IU3ABC IO91')).toEqual({ tag: 'POTA', call: 'IU3ABC', base: 'IU3ABC' });
    expect(parseSpecialCq('CQ SOTA IU3ABC IO91wm')).toEqual({ tag: 'SOTA', call: 'IU3ABC', base: 'IU3ABC' });
    expect(parseSpecialCq('cq iota iu3abc')).toEqual({ tag: 'IOTA', call: 'IU3ABC', base: 'IU3ABC' });
    expect(parseSpecialCq('CQ WWFF IU3ABC IO91')?.tag).toBe('WWFF');
    expect(parseSpecialCq('CQ QRP IU3ABC IO91')?.tag).toBe('QRP');
  });

  it('accetta il nominativo con la barra o fra parentesi angolari', () => {
    expect(parseSpecialCq('CQ POTA IU3ABC/P IO91')).toEqual({ tag: 'POTA', call: 'IU3ABC/P', base: 'IU3ABC' });
    expect(parseSpecialCq('CQ POTA <PJ4/IU3ABC>')).toEqual({ tag: 'POTA', call: 'PJ4/IU3ABC', base: 'IU3ABC' });
    expect(parseSpecialCq('  CQ   SOTA   <IU3ABC/P>  ')?.call).toBe('IU3ABC/P');
  });

  it('non riconosce gli altri CQ con direzione ne\' i messaggi che non sono CQ', () => {
    for (const text of ['CQ DX IU3ABC IO91', 'CQ NA IU3ABC IO91', 'CQ TEST IU3ABC IO91', 'CQ IU3ABC IO91', 'IU3ABC IK1XYZ POTA', 'CQ POTA', 'CQ POTA IO91']) {
      expect(parseSpecialCq(text), text).toBeNull();
    }
  });

  it('scarta parole in piu\' o locatori sbagliati', () => {
    expect(parseSpecialCq('CQ POTA IU3ABC IO91 73')).toBeNull();
    expect(parseSpecialCq('CQ POTA IU3ABC -12')).toBeNull();
    expect(parseSpecialCq('CQ POTA /IU3ABC IO91')).toBeNull();
    expect(parseSpecialCq('CQ POTA ABCDEF IO91')).toBeNull();
  });
});

describe('baseCall', () => {
  it('tiene il pezzo piu\' lungo', () => {
    expect(baseCall('IU3ABC/P')).toBe('IU3ABC');
    expect(baseCall('pj4/iu3abc')).toBe('IU3ABC');
    expect(baseCall('IU3ABC')).toBe('IU3ABC');
  });
});
