import { describe, expect, it } from 'vitest';
import { activeTx, txMessages } from './txMessages';

describe('txMessages', () => {
  it('builds the six standard FT8 messages for a QSO', () => {
    expect(txMessages({ myCall: 'AA1AA', myGrid: 'FN42ab', dxCall: 'BB2BB', report: '-10' })).toEqual([
      'BB2BB AA1AA FN42',
      'BB2BB AA1AA -10',
      'BB2BB AA1AA R-10',
      'BB2BB AA1AA RR73',
      'BB2BB AA1AA 73',
      'CQ AA1AA FN42',
    ]);
  });

  it('writes positive reports with a sign', () => {
    expect(txMessages({ myCall: 'AA1AA', myGrid: 'FN42', dxCall: 'BB2BB', report: '5' })[1]).toBe('BB2BB AA1AA +05');
    expect(txMessages({ myCall: 'AA1AA', myGrid: 'FN42', dxCall: 'BB2BB', report: '-3' })[2]).toBe('BB2BB AA1AA R-03');
  });

  it('leaves only the CQ when there is no DX station', () => {
    const m = txMessages({ myCall: 'AA1AA', myGrid: 'FN42', dxCall: '', report: '' });
    expect(m.slice(0, 5)).toEqual(['', '', '', '', '']);
    expect(m[5]).toBe('CQ AA1AA FN42');
  });

  it('defaults to a 00 report when none is set', () => {
    expect(txMessages({ myCall: 'AA1AA', myGrid: 'FN42', dxCall: 'BB2BB', report: '' })[1]).toBe('BB2BB AA1AA +00');
  });
});

describe('activeTx', () => {
  const messages = ['BB2BB AA1AA FN42', 'BB2BB AA1AA -10', '', '', '', 'CQ AA1AA FN42'];
  it('finds which line is being sent', () => {
    expect(activeTx(messages, 'BB2BB AA1AA -10')).toBe(1);
    expect(activeTx(messages, ' cq aa1aa fn42 ')).toBe(5);
  });
  it('is -1 when nothing matches or nothing is set', () => {
    expect(activeTx(messages, 'TNX 73')).toBe(-1);
    expect(activeTx(messages, undefined)).toBe(-1);
    expect(activeTx(messages, '')).toBe(-1);
  });
});
