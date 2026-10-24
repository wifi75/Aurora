import { describe, expect, it } from 'vitest';
import { COLOR_KEYS, applyColors, defaultColors, inkFor, readColors, saveColors } from './colors';

function memory(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return { data, getItem: (k: string) => data[k] ?? null, setItem: (k: string, v: string) => { data[k] = v; } };
}
const broken = { getItem: () => { throw new Error('no'); }, setItem: () => { throw new Error('no'); } };

describe('colors', () => {
  it('has a distinct default color for every kind of message', () => {
    const c = defaultColors();
    expect(Object.keys(c).sort()).toEqual([...COLOR_KEYS].sort());
    expect(new Set(Object.values(c)).size).toBe(COLOR_KEYS.length);
  });

  it('remembers custom colors and keeps defaults for the rest', () => {
    const store = memory();
    saveColors(store, { ...defaultColors(), new_dxcc: '#112233' });
    expect(readColors(store).new_dxcc).toBe('#112233');
    expect(readColors(store).cq).toBe(defaultColors().cq);
  });

  it('ignores invalid values, garbage and a broken storage', () => {
    expect(readColors(memory({ 'aurora-colors': '{"new_dxcc":"rosso","cq":"#12345"}' }))).toEqual(defaultColors());
    expect(readColors(memory({ 'aurora-colors': 'non json' }))).toEqual(defaultColors());
    expect(readColors(broken)).toEqual(defaultColors());
    expect(readColors(null)).toEqual(defaultColors());
    expect(() => saveColors(broken, defaultColors())).not.toThrow();
  });

  it('picks a readable text color', () => {
    expect(inkFor('#ffffff')).toBe('#000000');
    expect(inkFor('#000000')).toBe('#ffffff');
    expect(inkFor('#f2a93b')).toBe('#000000');
    expect(inkFor('#102040')).toBe('#ffffff');
  });

  it('publishes the colors as CSS variables', () => {
    const root = document.createElement('div');
    applyColors(root, { ...defaultColors(), new_dxcc_band: '#abcdef' });
    expect(root.style.getPropertyValue('--tag-new-dxcc-band')).toBe('#abcdef');
    expect(root.style.getPropertyValue('--tag-new-dxcc-band-ink')).toBe('#000000');
  });
});
