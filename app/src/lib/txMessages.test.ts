import { describe, expect, it } from 'vitest';
import { activeTx, classifyTx, txMessages } from './txMessages';

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

describe('classifyTx', () => {
  it('recognises the type of message whatever the calls and the report', () => {
    expect(classifyTx('KC2NJ IU3ABC JN55')).toBe(0);
    expect(classifyTx('KC2NJ IU3ABC -18')).toBe(1);
    expect(classifyTx('KC2NJ IU3ABC +05')).toBe(1);
    expect(classifyTx('KC2NJ IU3ABC R-19')).toBe(2);
    expect(classifyTx('KC2NJ IU3ABC RRR')).toBe(3);
    expect(classifyTx('KC2NJ IU3ABC RR73')).toBe(3);
    expect(classifyTx('KC2NJ IU3ABC 73')).toBe(4);
    expect(classifyTx('CQ IU3ABC IO91')).toBe(5);
    expect(classifyTx('CQ DX IU3ABC IO91')).toBe(5);
    expect(classifyTx('TNX 73')).toBe(-1);
    expect(classifyTx('')).toBe(-1);
  });

  it('lights the right row when the engine sends a report different from the one known', () => {
    const messages = txMessages({ myCall: 'IU3ABC', myGrid: 'IO91wm', dxCall: 'KC2NJ', report: '-18' });
    expect(activeTx(messages, 'KC2NJ IU3ABC R-21')).toBe(2);
    expect(activeTx(messages, 'KC2NJ IU3ABC -07')).toBe(1);
  });
});
