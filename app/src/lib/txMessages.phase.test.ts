import { describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { nextTx } from './txMessages';

const d = (id: number, text: string): DecodedMessage => ({ id, time_ms: id, snr: -10, delta_time: 0, freq_hz: 1500, text, tag: null });

describe('nextTx', () => {
  it('is Tx6 (CQ) when nobody is being called', () => {
    expect(nextTx([], 'AA1AA', '')).toBe(5);
  });

  it('is Tx1 while calling a station that has not answered yet', () => {
    expect(nextTx([d(2, 'CQ CC3CC FN42'), d(1, 'AA1AA DD4DD -10')], 'AA1AA', 'BB2BB')).toBe(0);
  });

  it('answers a locator with the report (Tx2)', () => {
    expect(nextTx([d(1, 'AA1AA BB2BB KK00')], 'AA1AA', 'BB2BB')).toBe(1);
  });

  it('answers a report with the R-report (Tx3)', () => {
    expect(nextTx([d(1, 'AA1AA BB2BB -12')], 'AA1AA', 'BB2BB')).toBe(2);
    expect(nextTx([d(1, 'AA1AA BB2BB +05')], 'AA1AA', 'BB2BB')).toBe(2);
  });

  it('answers an R-report with RR73 (Tx4)', () => {
    expect(nextTx([d(1, 'AA1AA BB2BB R-12')], 'AA1AA', 'BB2BB')).toBe(3);
  });

  it('answers RRR, RR73 and 73 with the final 73 (Tx5)', () => {
    for (const closing of ['RRR', 'RR73', '73']) expect(nextTx([d(1, `AA1AA BB2BB ${closing}`)], 'AA1AA', 'BB2BB')).toBe(4);
  });

  it('follows the most recent message from the station, newest first', () => {
    const list = [d(3, 'AA1AA BB2BB R-12'), d(2, 'AA1AA BB2BB -12'), d(1, 'AA1AA BB2BB KK00')];
    expect(nextTx(list, 'AA1AA', 'BB2BB')).toBe(3);
  });

  it('ignores messages addressed to someone else or sent by someone else', () => {
    expect(nextTx([d(2, 'CC3CC BB2BB -12'), d(1, 'AA1AA CC3CC -12')], 'AA1AA', 'BB2BB')).toBe(0);
  });

  it('is case-insensitive', () => {
    expect(nextTx([d(1, 'aa1aa bb2bb -12')], 'AA1AA', 'bb2bb')).toBe(2);
  });
});
