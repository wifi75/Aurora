import { describe, expect, it } from 'vitest';
import { qslShort } from './qslShort';

describe('qslShort', () => {
  it('keeps LoTW, OQRS and the QSL manager, short', () => {
    expect(qslShort('LoTW')).toBe('LoTW');
    expect(qslShort('M0OXO OQRS; LoTW')).toBe('via M0OXO · OQRS · LoTW');
    expect(qslShort('Club Log OQRS')).toBe('OQRS');
    expect(qslShort('EB7DX')).toBe('via EB7DX');
    expect(qslShort('DL2GAC (B/d)')).toBe('via DL2GAC');
    expect(qslShort('IU3ABC/P direct')).toBe('via IU3ABC/P');
  });

  it('home call, bureau and empty', () => {
    expect(qslShort('Home Call')).toBe('Home Call');
    expect(qslShort('h/c')).toBe('h/c');
    expect(qslShort('Bureau')).toBe('Bureau');
    expect(qslShort('  ')).toBe('');
  });
});
