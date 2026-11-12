import { describe, expect, it } from 'vitest';
import { clearMemory, DEFAULT_SCAN, MEM_KEY, memorySummary, readMemories, readScan, SCAN_KEY, writeMemories, writeScan } from './digiScan';

function memoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() {
      return data.size;
    },
    clear: () => data.clear(),
    getItem: (k) => data.get(k) ?? null,
    key: (i) => [...data.keys()][i] ?? null,
    removeItem: (k) => void data.delete(k),
    setItem: (k, v) => void data.set(k, v),
  };
}

describe('digi scan and memories', () => {
  it('reads the defaults when nothing is saved or the data is broken, and clamps the dwell', () => {
    const s = memoryStorage();
    expect(readScan(s)).toEqual(DEFAULT_SCAN);
    s.setItem(SCAN_KEY, '{nope');
    expect(readScan(s)).toEqual(DEFAULT_SCAN);
    s.setItem(SCAN_KEY, JSON.stringify({ dwell: 1, once: true, timeMarks: 'x' }));
    expect(readScan(s)).toEqual({ dwell: 2, once: true, timeMarks: true, freqMarks: true });
    writeScan(s, { dwell: 30, once: false, timeMarks: false, freqMarks: false });
    expect(readScan(s)).toEqual({ dwell: 30, once: false, timeMarks: false, freqMarks: false });
  });

  it('clears one memory and keeps the others', () => {
    const s = memoryStorage();
    writeMemories(s, { '1': { mode: 'rtty', rx_freq_hz: 1500, rtty_shift_hz: 170, rtty_baud: 45.45 }, '2': { mode: 'psk31', rx_freq_hz: 1000 } });
    clearMemory(s, 1);
    expect(Object.keys(readMemories(s))).toEqual(['2']);
    s.setItem(MEM_KEY, '[]');
    expect(readMemories(s)).toEqual({});
  });

  it('summarises a memory with mode, frequencies and the RTTY data', () => {
    expect(memorySummary({ mode: 'rtty', rx_freq_hz: 1500.4, tx_follow_rx: true, rtty_shift_hz: 170, rtty_baud: 45.45 })).toBe('RTTY · Rx 1500 Hz · 170 Hz / 45.45 bd');
    expect(memorySummary({ mode: 'cw', rx_freq_hz: 700, tx_follow_rx: false, tx_freq_hz: 800 })).toBe('CW · Rx 700 Hz · Tx 800 Hz');
  });
});
