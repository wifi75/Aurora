import { describe, expect, it } from 'vitest';
import { SERIAL_KEY, commentWithExchange, exchangeSent, formatSerial, readSerial, writeSerial } from './contestSerial';

const mem = () => {
  const m = new Map<string, string>();
  return { getItem: (k: string) => m.get(k) ?? null, setItem: (k: string, v: string) => void m.set(k, v), m };
};

describe('numero progressivo dei contest', () => {
  it('parte da 1 e si ricorda', () => {
    const s = mem();
    expect(readSerial(s)).toBe(1);
    writeSerial(s, readSerial(s) + 1);
    expect(s.m.get(SERIAL_KEY)).toBe('2');
    expect(readSerial(s)).toBe(2);
  });
  it('valori non validi tornano a 1', () => {
    const s = mem();
    s.setItem(SERIAL_KEY, 'abc');
    expect(readSerial(s)).toBe(1);
    expect(writeSerial(s, 0)).toBe(1);
    expect(writeSerial(s, 2.5)).toBe(1);
    expect(readSerial(null)).toBe(1);
  });
  it('tre cifre almeno', () => {
    expect(formatSerial(1)).toBe('001');
    expect(formatSerial(42)).toBe('042');
    expect(formatSerial(1234)).toBe('1234');
  });
  it('scambio da mandare: scritto o il numero', () => {
    expect(exchangeSent('', 7)).toBe('007');
    expect(exchangeSent(' 15 ', 7)).toBe('15');
  });
  it('gli scambi vanno nelle note solo se ricevuti', () => {
    expect(commentWithExchange('ciao', '001', '015')).toBe('ciao NRS 001 NRR 015');
    expect(commentWithExchange('', '001', '')).toBe('');
  });
});
