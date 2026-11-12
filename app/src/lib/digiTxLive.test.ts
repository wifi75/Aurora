import { describe, expect, it } from 'vitest';
import { afterTransmission, charCount, planEdit } from './digiTxLive';

describe('finestra di trasmissione dal vivo', () => {
  it('quello che si scrive in fondo si accoda', () => {
    expect(planEdit('CQ ', 'CQ DE', { queued: 3, sent: 1 })).toEqual({ from: 3, text: 'DE' });
  });

  it('il Backspace toglie dalla coda solo quello che non e partito', () => {
    expect(planEdit('CQ DE', 'CQ D', { queued: 5, sent: 2 })).toEqual({ from: 4, text: '' });
  });

  it('si corregge anche a meta della parte non partita', () => {
    expect(planEdit('CQ DE IU3ABC PSE K', 'CQ DE IU3ABC 599 K', { queued: 18, sent: 4 })).toEqual({ from: 13, text: '599 K' });
  });

  it('i caratteri gia partiti non si toccano', () => {
    expect(planEdit('CQ DE', 'C', { queued: 5, sent: 3 })).toBeNull();
    expect(planEdit('CQ DE', 'XQ DE', { queued: 5, sent: 1 })).toBeNull();
  });

  it('a fine trasmissione resta solo il testo non partito', () => {
    expect(afterTransmission('CQ DE IU3ABC bozza', { queued: 12, sent: 12 }, true)).toBe(' bozza');
    expect(afterTransmission('CQ DE IU3ABC', { queued: 12, sent: 3 }, false)).toBe('DE IU3ABC');
  });

  it('i caratteri si contano come li conta il motore', () => {
    expect(charCount('CQ')).toBe(2);
  });
});
