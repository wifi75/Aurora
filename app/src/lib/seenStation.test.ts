import { describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { callConfirmText, isStationSeen, needsCallConfirm } from './seenStation';

const NOW = 1_000_000_000_000;
const dec = (id: number, text: string, agoMs: number): DecodedMessage => ({ id, time_ms: NOW - agoMs, snr: -10, delta_time: 0, freq_hz: 1000, text, tag: null });

describe('stazione vista', () => {
  const recent = [dec(1, 'CQ K1ABC FN42', 60_000), dec(2, 'IU3ABC K1XYZ -05', 120_000)];
  it('e vista se compare tra i decode recenti', () => {
    expect(isStationSeen(recent, 'K1ABC', NOW)).toBe(true);
    expect(isStationSeen(recent, 'K1XYZ', NOW)).toBe(true);
  });
  it('il confronto ignora il portatile e i prefissi', () => {
    expect(isStationSeen(recent, 'K1ABC/P', NOW)).toBe(true);
    expect(isStationSeen(recent, 'F/K1ABC', NOW)).toBe(true);
  });
  it('non e vista se manca o e piu vecchia di 15 minuti', () => {
    expect(isStationSeen(recent, 'XX9BB', NOW)).toBe(false);
    expect(isStationSeen([dec(3, 'CQ XX9BB OL62', 16 * 60_000)], 'XX9BB', NOW)).toBe(false);
  });
});

describe('conferma prima di chiamare', () => {
  const recent = [dec(1, 'CQ K1ABC FN42', 60_000)];
  it('non chiede se la stazione e tra i decode recenti', () => {
    expect(needsCallConfirm(true, recent, 'K1ABC', NOW)).toBe(false);
  });
  it('chiede se la stazione manca', () => {
    expect(needsCallConfirm(true, recent, 'XX9BB', NOW)).toBe(true);
  });
  it('non chiede con l interruttore spento', () => {
    expect(needsCallConfirm(false, recent, 'XX9BB', NOW)).toBe(false);
  });
  it('il testo ha nominativo e paese, senza origine', () => {
    expect(callConfirmText('Vuoi chiamare {call}?', 'XX9BB', 'Macao')).toBe('Vuoi chiamare XX9BB (Macao)?');
    expect(callConfirmText('Vuoi chiamare {call}?', 'XX9BB', null)).toBe('Vuoi chiamare XX9BB?');
  });
});
