import { describe, expect, it } from 'vitest';
import { formatKhz, presetAt, wefaxGroups, wefaxPresets } from './wefaxStations';

describe('stazioni WEFAX', () => {
  it('ordina per banda e poi per frequenza', () => {
    const groups = wefaxGroups();
    expect(groups.map((g) => g.band)).toEqual([...groups.map((g) => g.band)].sort((a, b) => a - b));
    for (const g of groups) {
      const f = g.presets.map((p) => p.khz);
      expect(f).toEqual([...f].sort((a, b) => a - b));
      expect(g.presets.every((p) => p.band === g.band)).toBe(true);
    }
  });
  it('la radio va 1,9 kHz sotto la frequenza assegnata (USB)', () => {
    const p = wefaxPresets().find((x) => x.khz === 7880);
    expect(p?.dialHz).toBe(7_878_100);
    expect(wefaxPresets().find((x) => x.khz === 13882.5)?.dialHz).toBe(13_880_600);
  });
  it('riconosce il preset su cui e\' la radio', () => {
    expect(presetAt(3_853_100)?.call).toContain('DDH3');
    expect(presetAt(14_074_000)).toBeNull();
    expect(presetAt(0)).toBeNull();
  });
  it('scrive le frequenze con il separatore delle migliaia', () => {
    expect(formatKhz(3855)).toBe('3 855');
    expect(formatKhz(13882.5)).toBe('13 882,5');
  });
});
