import { describe, expect, it } from 'vitest';
import { cleanExtra, composeReceived, composeSent, extraLabelKey, hasExchMultiplier, readSection, writeSection } from './contestSection';

function memory() {
  const m = new Map<string, string>();
  return { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v), removeItem: (k: string) => void m.delete(k) };
}

describe('scambio con terzo elemento', () => {
  it('compone lo scambio ricevuto con il numero e la sezione', () => {
    expect(composeReceived('003', 'acc')).toBe('003 acc');
    expect(composeReceived(' 003 ', '')).toBe('003');
    expect(composeReceived('', 'ACC')).toBe('ACC');
    expect(composeReceived(undefined, undefined)).toBe('');
  });
  it('compone lo scambio inviato solo se alla stazione tocca la sezione', () => {
    expect(composeSent('001', 'acc', true)).toBe('001 ACC');
    expect(composeSent('001', 'ACC', false)).toBe('001');
    expect(composeSent('001', '', true)).toBe('001');
  });
  it('pulisce la sezione', () => {
    expect(cleanExtra(' ny  ')).toBe('NY');
    expect(cleanExtra('abcdefghijklmnopqrstuvwxyz')).toBe('ABCDEFGHIJKLMNOPQRST');
  });
  it('sceglie il nome della casella dal contest', () => {
    expect(extraLabelKey('UBA CW CONTEST')).toBe('ssbc.exLabel.uba');
    expect(extraLabelKey('ARRL 10 meter Contest')).toBe('ssbc.exLabel.arrl');
    expect(extraLabelKey('CQ WW')).toBe('ssbc.exLabel.generic');
    expect(extraLabelKey(null)).toBe('ssbc.exLabel.generic');
  });
  it('ricorda la propria sezione per contest', () => {
    const s = memory();
    writeSection(s, 'UBA CW contest', 'acc');
    expect(readSection(s, 'UBA CW contest')).toBe('ACC');
    expect(readSection(s, 'Altro')).toBe('');
    writeSection(s, 'UBA CW contest', '');
    expect(readSection(s, 'UBA CW contest')).toBe('');
    expect(readSection(null, 'x')).toBe('');
  });
  it('riconosce il moltiplicatore exch nelle regole', () => {
    expect(hasExchMultiplier('points=any=3\nmult=exch/band/if:dxcc:K,VE; dxcc/band')).toBe(true);
    expect(hasExchMultiplier('mult=dxcc; exch/band/skip:XXX')).toBe(true);
    expect(hasExchMultiplier('points=any=1\nmult=dxcc/band; cq')).toBe(false);
    expect(hasExchMultiplier(undefined)).toBe(false);
  });
});
