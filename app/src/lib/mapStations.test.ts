import { describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { stationsWithGrid } from './mapStations';

const d = (id: number, text: string, snr = -10): DecodedMessage => ({ id, time_ms: id, snr, delta_time: 0, freq_hz: 1500, text, tag: null });

describe('stationsWithGrid', () => {
  it('reads the sender and the locator from CQs and from replies', () => {
    const list = stationsWithGrid([d(1, 'CQ K1ABC FN42'), d(2, 'K1ABC G0XYZ IO91'), d(3, 'CQ DX JA1AA PM95')]);
    expect(list.map((s) => [s.call, s.grid, s.cq])).toEqual([['K1ABC', 'FN42', true], ['G0XYZ', 'IO91', false], ['JA1AA', 'PM95', true]]);
  });

  it('keeps one station per callsign, the latest message wins', () => {
    const list = stationsWithGrid([d(5, 'CQ K1ABC FN42', -5), d(2, 'CQ K1ABC FN41', -20)]);
    expect(list).toHaveLength(1);
    expect(list[0]).toMatchObject({ grid: 'FN42', snr: -5 });
  });

  it('ignores RR73, reports, 73 and messages without a locator or with a free text', () => {
    const list = stationsWithGrid([d(1, 'K1ABC G0XYZ RR73'), d(2, 'K1ABC G0XYZ -12'), d(3, 'K1ABC G0XYZ 73'), d(4, 'TNX 73'), d(5, 'CQ TEST FN42'), d(6, '')]);
    expect(list).toEqual([]);
  });
});
