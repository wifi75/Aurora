import { beforeEach, describe, expect, it } from 'vitest';
import type { DecodedMessage, StationStatus } from '../api/schemas';
import { useStation } from './station';

const status: StationStatus = {
  dial_hz: 14_074_000, mode: 'FT8', tx_enabled: true, transmitting: false, decoding: true,
  de_call: 'AA1AA', de_grid: 'FN42',
};

const decode = (id: number): DecodedMessage => ({
  id, time_ms: id, snr: -10, delta_time: 0, freq_hz: 1500, text: `CQ TEST ${id}`, tag: null,
});

describe('useStation', () => {
  beforeEach(() => useStation.getState().reset());

  it('apply status updates station', () => {
    useStation.getState().apply({ type: 'status', ...status });
    expect(useStation.getState().status?.dial_hz).toBe(14_074_000);
  });

  it('apply decode keeps newest first', () => {
    const { apply } = useStation.getState();
    apply({ type: 'decode', ...decode(1) });
    apply({ type: 'decode', ...decode(2) });
    expect(useStation.getState().decodes.map((d) => d.id)).toEqual([2, 1]);
  });

  it('apply decode caps at 200', () => {
    const { apply } = useStation.getState();
    for (let i = 1; i <= 250; i++) apply({ type: 'decode', ...decode(i) });
    const list = useStation.getState().decodes;
    expect(list).toHaveLength(200);
    expect(list[0]?.id).toBe(250);
    expect(list[199]?.id).toBe(51);
  });

  it('tx_halted clears transmitting without waiting for a new status', () => {
    const { apply } = useStation.getState();
    apply({ type: 'status', ...status, transmitting: true });
    apply({ type: 'tx_halted' });
    expect(useStation.getState().status?.transmitting).toBe(false);
  });

  it('keeps the connection state', () => {
    useStation.getState().setConnection('open');
    expect(useStation.getState().connection).toBe('open');
  });
});
