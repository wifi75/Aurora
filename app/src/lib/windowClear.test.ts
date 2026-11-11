import { describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { afterMark, markNow, NO_MARK } from './windowClear';

const msg = (id: number): DecodedMessage => ({ id, time_ms: 0, snr: 0, delta_time: 0, freq_hz: 1000, text: 'CQ IU3ABC IO91', tag: null });

describe('pulizia di una sola finestra', () => {
  it('senza pulizia non toglie nulla', () => {
    const list = [msg(2), msg(1)];
    expect(afterMark(list, NO_MARK)).toBe(list);
  });

  it('toglie le decodifiche e le righe Tx gia presenti, tiene le nuove', () => {
    const mark = markNow([msg(5), msg(4)], [msg(-2), msg(-1)]);
    const next = [msg(7), msg(-3), msg(5), msg(-2), msg(4)];
    expect(afterMark(next, mark).map((d) => d.id)).toEqual([7, -3]);
  });

  it('due finestre hanno segni indipendenti', () => {
    const band = markNow([msg(3)], []);
    const rx = NO_MARK;
    const list = [msg(3), msg(2)];
    expect(afterMark(list, band)).toEqual([]);
    expect(afterMark(list, rx)).toHaveLength(2);
  });
});
