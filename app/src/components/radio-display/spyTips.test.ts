import { describe, expect, it } from 'vitest';
import { STRINGS } from '../../i18n';
import { foxStrings } from '../../i18n/strings/fox';
import { tooltipStrings } from '../../i18n/strings/tooltips';
import { spyTipKey } from './instruments';

describe('suggerimenti delle spie del display', () => {
  it('riconoscono la spia dalla prima parola della scritta', () => {
    expect(spyTipKey('RX')).toBe('tip.rd.rx');
    expect(spyTipKey('TX EN')).toBe('tip.rd.txEn');
    expect(spyTipKey('SWR 1.5')).toBe('tip.rd.swr');
    expect(spyTipKey('CLAR +20')).toBe('tip.rd.clar');
    expect(spyTipKey('VFO-B')).toBe('tip.rd.vfoB');
    expect(spyTipKey('S9+10')).toBe('tip.rd.s');
    expect(spyTipKey('14.074')).toBeNull();
  });
});

describe('i nuovi suggerimenti', () => {
  const tips = Object.entries({ ...tooltipStrings, ...foxStrings }).filter(([k]) => k.startsWith('tip.'));
  it('hanno sei lingue non vuote e sono registrati', () => {
    expect(tips.length).toBeGreaterThan(100);
    for (const [key, entry] of tips) {
      expect(entry, key).toHaveLength(6);
      for (const text of entry) expect(text.trim(), key).not.toBe('');
      expect(STRINGS[key as keyof typeof STRINGS], key).toBe(entry);
    }
  });
});
