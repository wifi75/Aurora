import { describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { cqDecodes } from './selectors';

const decode = (id: number, tag: DecodedMessage['tag']): DecodedMessage => ({
  id, time_ms: id, snr: -10, delta_time: 0, freq_hz: 1500, text: `msg ${id}`, tag,
});

describe('cqDecodes', () => {
  it('keeps CQ, new DXCC and new zone messages in the same order', () => {
    const list = [decode(5, 'cq'), decode(4, null), decode(3, 'new_dxcc'), decode(2, 'auto'), decode(1, 'new_zone')];
    expect(cqDecodes(list).map((d) => d.id)).toEqual([5, 3, 1]);
  });

  it('limits the number of messages', () => {
    const list = Array.from({ length: 100 }, (_, i) => decode(i + 1, 'cq'));
    expect(cqDecodes(list, 40)).toHaveLength(40);
    expect(cqDecodes(list, 40)[0]?.id).toBe(1);
  });

  it('is empty when there is no CQ', () => {
    expect(cqDecodes([decode(1, null)])).toEqual([]);
    expect(cqDecodes([])).toEqual([]);
  });
});
