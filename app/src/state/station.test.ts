import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { DecodedMessage, StationStatus } from '../api/schemas';
import { usePrefs } from '../lib/prefs';
import { bandChanged, bandOrModeChanged, useStation } from './station';

const status: StationStatus = {
  dial_hz: 14_074_000, mode: 'FT8', tx_enabled: true, transmitting: false, decoding: true,
  de_call: 'AA1AA', de_grid: 'FN42',
};

const decode = (id: number): DecodedMessage => ({
  id, time_ms: id, snr: -10, delta_time: 0, freq_hz: 1500, text: `CQ TEST ${id}`, tag: null,
});

describe('useStation', () => {
  beforeEach(() => useStation.getState().reset());
  afterEach(() => vi.useRealTimers());

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

  it('keeps a commanded change over the old states of the engine until it reports the same value', () => {
    const { apply, patchStatus } = useStation.getState();
    apply({ type: 'status', ...status });
    patchStatus({ mode: 'FT4', dial_hz: 7_047_500 });
    apply({ type: 'status', ...status }); // stato vecchio: non rimette indietro il pulsante
    expect(useStation.getState().status?.mode).toBe('FT4');
    expect(useStation.getState().status?.dial_hz).toBe(7_047_500);
    apply({ type: 'status', ...status, mode: 'FT4' }); // il modo e' confermato, la frequenza no
    expect(useStation.getState().pending).toEqual({ dial_hz: 7_047_500 });
    apply({ type: 'status', ...status, mode: 'FT4', dial_hz: 7_047_500 });
    expect(useStation.getState().pending).toEqual({});
    apply({ type: 'status', ...status, mode: 'FT8' }); // da qui in poi vale quello che dice il motore
    expect(useStation.getState().status?.mode).toBe('FT8');
  });

  it('gives up on a commanded change the engine never confirms', () => {
    vi.useFakeTimers();
    try {
      const { apply, patchStatus } = useStation.getState();
      apply({ type: 'status', ...status });
      patchStatus({ mode: 'FT4' });
      vi.advanceTimersByTime(9_000);
      apply({ type: 'status', ...status });
      expect(useStation.getState().status?.mode).toBe('FT8');
    } finally {
      vi.useRealTimers();
    }
  });

  it('puts the old value back when the command fails', () => {
    const { apply, patchStatus, revertStatus } = useStation.getState();
    apply({ type: 'status', ...status });
    patchStatus({ mode: 'FT4' });
    revertStatus(['mode'], { mode: 'FT8' });
    expect(useStation.getState().status?.mode).toBe('FT8');
    apply({ type: 'status', ...status });
    expect(useStation.getState().status?.mode).toBe('FT8');
  });

  it('lights the chosen band at once and drops it when the engine is on that band', () => {
    const { apply, setPendingBand } = useStation.getState();
    apply({ type: 'status', ...status });
    setPendingBand('40m');
    apply({ type: 'status', ...status }); // ancora sulla banda di prima
    expect(useStation.getState().pendingBand).toBe('40m');
    apply({ type: 'status', ...status, dial_hz: 7_074_000 });
    expect(useStation.getState().pendingBand).toBeNull();
  });

  it('clears the band activity when the band or the mode changes, if the option is on', () => {
    const { apply } = useStation.getState();
    apply({ type: 'status', ...status });
    apply({ type: 'decode', ...decode(1) });
    apply({ type: 'status', ...status, dial_hz: 14_075_000 }); // stessa banda: resta
    expect(useStation.getState().decodes).toHaveLength(1);
    apply({ type: 'status', ...status, dial_hz: 7_074_000 }); // un'altra banda: si svuota
    expect(useStation.getState().decodes).toHaveLength(0);
    apply({ type: 'decode', ...decode(2) });
    apply({ type: 'status', ...status, dial_hz: 7_074_000, mode: 'FT4' }); // un altro modo: si svuota
    expect(useStation.getState().decodes).toHaveLength(0);
  });

  it('clears at once when the change is commanded from Aurora, and only once', () => {
    const { apply, patchStatus } = useStation.getState();
    apply({ type: 'status', ...status });
    apply({ type: 'decode', ...decode(1) });
    patchStatus({ dial_hz: 7_074_000 });
    expect(useStation.getState().decodes).toHaveLength(0);
    apply({ type: 'decode', ...decode(2) });
    apply({ type: 'status', ...status, dial_hz: 7_074_000 }); // il motore conferma: non si svuota di nuovo
    expect(useStation.getState().decodes.map((d) => d.id)).toEqual([2]);
  });

  it('keeps the list when the option is off and never clears on the first status', () => {
    usePrefs.getState().set({ clearOnChange: false });
    try {
      const { apply } = useStation.getState();
      apply({ type: 'status', ...status });
      apply({ type: 'decode', ...decode(1) });
      apply({ type: 'status', ...status, mode: 'FT4' }); // solo il modo: l'opzione spenta le lascia
      expect(useStation.getState().decodes).toHaveLength(1);
      apply({ type: 'status', ...status, mode: 'FT4', dial_hz: 7_047_500 }); // la banda: si svuota sempre
      expect(useStation.getState().decodes).toHaveLength(0);
    } finally {
      usePrefs.getState().set({ clearOnChange: true });
    }
    useStation.getState().reset();
    useStation.getState().apply({ type: 'decode', ...decode(5) });
    useStation.getState().apply({ type: 'status', ...status });
    expect(useStation.getState().decodes).toHaveLength(1);
  });

  it('a band change empties the lists and the DX station, but a move inside the band does not', () => {
    const { apply, setPendingDx } = useStation.getState();
    apply({ type: 'status', ...status, dx_call: 'K1ABC', dx_grid: 'FN42', report: '-10' });
    apply({ type: 'decode', ...decode(1) });
    setPendingDx('K1ABC');
    apply({ type: 'status', ...status, dial_hz: 14_075_000, dx_call: 'K1ABC', dx_grid: 'FN42' });
    expect(useStation.getState().decodes).toHaveLength(1);
    expect(useStation.getState().status?.dx_call).toBe('K1ABC');
    apply({ type: 'status', ...status, dial_hz: 7_074_000, dx_call: 'K1ABC', dx_grid: 'FN42', report: '-10' });
    const s = useStation.getState();
    expect(s.decodes).toHaveLength(0);
    expect(s.txLog).toHaveLength(0);
    expect(s.pendingDx).toBeNull();
    expect([s.status?.dx_call, s.status?.dx_grid, s.status?.report]).toEqual([null, null, null]);
    // uno stato in ritardo con il vecchio chiamante non lo riporta, uno con un chiamante nuovo si
    apply({ type: 'status', ...status, dial_hz: 7_074_000, dx_call: 'K1ABC' });
    expect(useStation.getState().status?.dx_call).toBeNull();
    apply({ type: 'status', ...status, dial_hz: 7_074_000, dx_call: 'W2XYZ' });
    expect(useStation.getState().status?.dx_call).toBe('W2XYZ');
  });

  it('the band_changed event from the engine clears everything once, even without a status on the new band', () => {
    const { apply } = useStation.getState();
    apply({ type: 'status', ...status, dx_call: 'K1ABC' });
    apply({ type: 'decode', ...decode(1) });
    apply({ type: 'band_changed', band: '40m', dial_hz: 7_074_000, epoch: 1 });
    expect(useStation.getState().decodes).toHaveLength(0);
    expect(useStation.getState().status?.dx_call).toBeNull();
    // i messaggi della banda nuova restano: lo stato che conferma la banda e un secondo avviso non svuotano di nuovo
    apply({ type: 'decode', ...decode(2) });
    apply({ type: 'status', ...status, dial_hz: 7_074_000 });
    apply({ type: 'band_changed', band: '40m', dial_hz: 7_074_000, epoch: 1 });
    expect(useStation.getState().decodes.map((d) => d.id)).toEqual([2]);
    // e un'altra banda svuota di nuovo
    apply({ type: 'band_changed', band: '20m', dial_hz: 14_074_000, epoch: 2 });
    expect(useStation.getState().decodes).toHaveLength(0);
  });

  it('a band change that the status saw first is not cleared a second time by the event', () => {
    const { apply } = useStation.getState();
    apply({ type: 'status', ...status });
    apply({ type: 'status', ...status, dial_hz: 7_074_000 });
    apply({ type: 'decode', ...decode(3) });
    apply({ type: 'band_changed', band: '40m', dial_hz: 7_074_000, epoch: 1 });
    expect(useStation.getState().decodes.map((d) => d.id)).toEqual([3]);
  });

  it('bandChanged needs both bands to be known', () => {
    expect(bandChanged(null, status)).toBe(false);
    expect(bandChanged({ ...status, dial_hz: 0 }, status)).toBe(false);
    expect(bandChanged(status, { ...status, dial_hz: 14_080_000 })).toBe(false);
    expect(bandChanged(status, { ...status, dial_hz: 7_074_000 })).toBe(true);
  });

  it('bandOrModeChanged ignores unknown frequencies and modes', () => {
    expect(bandOrModeChanged(null, status)).toBe(false);
    expect(bandOrModeChanged({ ...status, dial_hz: 0 }, status)).toBe(false);
    expect(bandOrModeChanged({ ...status, mode: '' }, { ...status, mode: 'FT4' })).toBe(false);
    expect(bandOrModeChanged(status, { ...status, mode: 'FT4' })).toBe(true);
    expect(bandOrModeChanged(status, { ...status, dial_hz: 21_074_000 })).toBe(true);
  });

  it('keeps the wanted alerts, newest first, and the banner until it is dismissed', () => {
    const { apply, dismissWanted } = useStation.getState();
    const alert = { type: 'wanted' as const, call: 'K1ABC', reason: 'call', source: 'decode', text: 'CQ K1ABC FN42', time_ms: 1 };
    apply(alert);
    apply({ ...alert, call: '3Y0J', time_ms: 2 });
    expect(useStation.getState().wanted.map((w) => w.call)).toEqual(['3Y0J', 'K1ABC']);
    expect(useStation.getState().wantedBanner?.call).toBe('3Y0J');
    dismissWanted();
    expect(useStation.getState().wantedBanner).toBeNull();
    expect(useStation.getState().wanted).toHaveLength(2);
    useStation.getState().reset();
    expect(useStation.getState().wanted).toEqual([]);
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
