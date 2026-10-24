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

  it('logs what we transmit once per transmission, newest first, with negative ids', () => {
    const { apply } = useStation.getState();
    apply({ type: 'status', ...status, transmitting: true, tx_message: 'CQ AA1AA FN42', tx_df: 1200 });
    apply({ type: 'status', ...status, transmitting: true, tx_message: 'CQ AA1AA FN42', tx_df: 1200 });
    expect(useStation.getState().txLog).toHaveLength(1);
    apply({ type: 'status', ...status, transmitting: false, tx_message: 'CQ AA1AA FN42' });
    apply({ type: 'status', ...status, transmitting: true, tx_message: 'CQ AA1AA FN42', tx_df: 1200 });
    apply({ type: 'status', ...status, transmitting: true, tx_message: 'XY9ZZ AA1AA 73', tx_df: 1200 });
    const log = useStation.getState().txLog;
    expect(log.map((d) => d.text)).toEqual(['XY9ZZ AA1AA 73', 'CQ AA1AA FN42', 'CQ AA1AA FN42']);
    expect(log.every((d) => d.id < 0)).toBe(true);
    expect(new Set(log.map((d) => d.id)).size).toBe(3);
    expect(log[0]?.freq_hz).toBe(1200);
  });

  it('does not log when idle or without a message, and reset clears the log', () => {
    const { apply } = useStation.getState();
    apply({ type: 'status', ...status, transmitting: false, tx_message: 'CQ AA1AA FN42' });
    apply({ type: 'status', ...status, transmitting: true, tx_message: '  ' });
    expect(useStation.getState().txLog).toHaveLength(0);
    apply({ type: 'status', ...status, transmitting: true, tx_message: 'CQ AA1AA FN42' });
    useStation.getState().reset();
    expect(useStation.getState().txLog).toHaveLength(0);
  });

  it('patchStatus shows a change at once, before the engine reports it, and ignores it without a status', () => {
    useStation.getState().patchStatus({ dial_hz: 1 });
    expect(useStation.getState().status).toBeNull();
    useStation.getState().apply({ type: 'status', ...status });
    useStation.getState().patchStatus({ dial_hz: 14_074_000, mode: 'FT4' });
    expect(useStation.getState().status?.dial_hz).toBe(14_074_000);
    expect(useStation.getState().status?.mode).toBe('FT4');
    expect(useStation.getState().status?.de_call).toBe('AA1AA');
  });

  it('remembers the station picked with a double click until the engine reports its own DX', () => {
    const { apply, setPendingDx } = useStation.getState();
    setPendingDx('BB2BB');
    expect(useStation.getState().pendingDx).toBe('BB2BB');
    apply({ type: 'status', ...status, dx_call: null });
    expect(useStation.getState().pendingDx).toBe('BB2BB');
    apply({ type: 'status', ...status, dx_call: 'BB2BB' });
    expect(useStation.getState().pendingDx).toBeNull();
    setPendingDx('CC3CC');
    useStation.getState().reset();
    expect(useStation.getState().pendingDx).toBeNull();
  });
});
