import { describe, expect, it } from 'vitest';
import { mainCall, splitUnsure } from './digiBand';
import type { DigiSignal } from '../api/schemas';

const sig = (calls: DigiSignal['calls']): DigiSignal => ({ id: 1, freq_hz: 700, snr_db: 8, text: '', unsure: [], calls, cq: false });

describe('digiBand', () => {
  it('divide il testo in pezzi sicuri e incerti (gli spazi non sono mai incerti)', () => {
    expect(splitUnsure('CQ DE IU3ABC', [1, 2, 6, 7])).toEqual([
      { text: 'C', unsure: false },
      { text: 'Q', unsure: true },
      { text: ' DE ', unsure: false },
      { text: 'IU', unsure: true },
      { text: '3ABC', unsure: false },
    ]);
    expect(splitUnsure('', [])).toEqual([]);
  });

  it('mostra chi chiama CQ, poi chi trasmette, poi il piu probabile', () => {
    const other = { call: 'K1ABC', role: 'other', verified: true };
    const de = { call: 'DL1ABC', role: 'de', verified: true };
    const cq = { call: 'IU3ABC', role: 'cq', verified: false };
    expect(mainCall(sig([other, de, cq]))?.call).toBe('IU3ABC');
    expect(mainCall(sig([other, de]))?.call).toBe('DL1ABC');
    expect(mainCall(sig([other]))?.call).toBe('K1ABC');
    expect(mainCall(sig([]))).toBeNull();
  });
});
