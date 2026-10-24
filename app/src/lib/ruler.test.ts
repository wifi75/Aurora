import { describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { decodeMarkers, rulerTicks } from './ruler';

const decode = (over: Partial<DecodedMessage>): DecodedMessage => ({
  id: 1, time_ms: 100, snr: -10, delta_time: 0, freq_hz: 1500, text: 'CQ AA1AA FN42', tag: null, ...over,
});

describe('rulerTicks', () => {
  it('gives a tick every 500 Hz from 0 to 3000', () => {
    const ticks = rulerTicks(0, 3000);
    expect(ticks).toHaveLength(7);
    expect(ticks[0]).toEqual({ hz: 0, left: 0 });
    expect(ticks[3]).toEqual({ hz: 1500, left: 50 });
    expect(ticks[6]).toEqual({ hz: 3000, left: 100 });
  });

  it('honors a different step', () => {
    expect(rulerTicks(0, 3000, 1000).map((t) => t.hz)).toEqual([0, 1000, 2000, 3000]);
  });

  it('starts at the first multiple of the step inside the range', () => {
    expect(rulerTicks(200, 1200).map((t) => t.hz)).toEqual([500, 1000]);
  });

  it('is empty for an invalid range or step', () => {
    expect(rulerTicks(3000, 3000)).toEqual([]);
    expect(rulerTicks(3000, 0)).toEqual([]);
    expect(rulerTicks(0, 3000, 0)).toEqual([]);
    expect(rulerTicks(100, 200)).toEqual([]);
  });
});

describe('decodeMarkers', () => {
  it('keeps only the newest cycle and places each decode by frequency', () => {
    const markers = decodeMarkers(
      [
        decode({ id: 1, time_ms: 100, freq_hz: 600 }),
        decode({ id: 2, time_ms: 200, freq_hz: 1500 }),
        decode({ id: 3, time_ms: 200, freq_hz: 750 }),
      ],
      3000,
    );
    expect(markers.map((m) => m.id)).toEqual([2, 3]);
    expect(markers[0]?.left).toBe(50);
    expect(markers[1]?.left).toBe(25);
  });

  it('classifies cq, rare and other messages', () => {
    const markers = decodeMarkers(
      [
        decode({ id: 1, tag: 'cq' }),
        decode({ id: 2, tag: 'new_dxcc' }),
        decode({ id: 3, tag: 'new_zone' }),
        decode({ id: 4, tag: null, text: 'BB2BB AA1AA RR73' }),
      ],
      3000,
    );
    expect(markers.map((m) => m.kind)).toEqual(['cq', 'rare', 'rare', 'other']);
  });

  it('clamps frequencies outside the range and handles no decodes', () => {
    expect(decodeMarkers([decode({ freq_hz: 9000 })], 3000)[0]?.left).toBe(100);
    expect(decodeMarkers([decode({ freq_hz: 0 })], 3000)[0]?.left).toBe(0);
    expect(decodeMarkers([], 3000)).toEqual([]);
  });
});
