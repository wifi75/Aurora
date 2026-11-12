import { describe, expect, it } from 'vitest';
import { afterTransmission, charCount, planEdit } from './digiTxLive';

describe('finestra di trasmissione dal vivo', () => {
  it('quello che si scrive in fondo si accoda', () => {
    expect(planEdit('CQ ', 'CQ DE', { queued: 3, sent: 1 })).toEqual({ box: 'CQ DE', unsend: 0, append: 'DE', keep: 3 });
  });

  it('il Backspace toglie dalla coda solo quello che non e partito', () => {
    expect(planEdit('CQ DE', 'CQ D', { queued: 5, sent: 2 })).toEqual({ box: 'CQ D', unsend: 1, append: '', keep: 4 });
  });

  it('i caratteri gia partiti non si toccano', () => {
    expect(planEdit('CQ DE', 'C', { queued: 5, sent: 3 }).box).toBe('CQ DE');
    expect(planEdit('CQ DE', 'XQ DE', { queued: 5, sent: 1 }).box).toBe('CQ DE');
  });

  it('a fine trasmissione resta solo il testo non partito', () => {
    expect(afterTransmission('CQ DE IU3ABC bozza', { queued: 12, sent: 12 }, true)).toBe(' bozza');
    expect(afterTransmission('CQ DE IU3ABC', { queued: 12, sent: 3 }, false)).toBe('DE IU3ABC');
  });

  it('i caratteri si contano come li conta il motore', () => {
    expect(charCount('CQ')).toBe(2);
  });
});
