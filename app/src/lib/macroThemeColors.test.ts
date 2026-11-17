import { describe, expect, it } from 'vitest';
import { MACRO_THEME_COLORS_KEY, readMacroColors, saveMacroColor, themeColor } from './macroThemeColors';

function memory(initial?: string) {
  const data = new Map<string, string>(initial === undefined ? [] : [[MACRO_THEME_COLORS_KEY, initial]]);
  return { getItem: (k: string) => data.get(k) ?? null, setItem: (k: string, v: string) => void data.set(k, v) };
}

describe('colori delle macro per tema', () => {
  it('senza scelta vale il colore della macro, in entrambi i temi', () => {
    const s = readMacroColors(memory());
    expect(themeColor(s, 'dark', 'F1', 0x112233)).toBe(0x112233);
    expect(themeColor(s, 'light', 'F1', 0x112233)).toBe(0x112233);
    expect(themeColor(s, 'light', 'F1', null)).toBeNull();
  });
  it('la scelta di un tema non cambia l\'altro', () => {
    const m = memory();
    saveMacroColor(m, 'light', 'F1', 0xaabbcc);
    const s = readMacroColors(m);
    expect(themeColor(s, 'light', 'F1', 0x112233)).toBe(0xaabbcc);
    expect(themeColor(s, 'dark', 'F1', 0x112233)).toBe(0x112233);
    saveMacroColor(m, 'dark', 'F1', 0x010203);
    const t = readMacroColors(m);
    expect(themeColor(t, 'light', 'F1', null)).toBe(0xaabbcc);
    expect(themeColor(t, 'dark', 'F1', null)).toBe(0x010203);
  });
  it('«nessun colore» scelto per un tema vale anche se la macro ne ha uno', () => {
    const m = memory();
    saveMacroColor(m, 'dark', 'F2', null);
    expect(themeColor(readMacroColors(m), 'dark', 'F2', 0xff0000)).toBeNull();
  });
  it('un archivio rovinato o con valori sbagliati si ignora', () => {
    expect(readMacroColors(memory('non json'))).toEqual({});
    expect(readMacroColors(memory(JSON.stringify({ light: { F1: 'rosso', F2: 5, F3: -1, F4: 99999999 } })))).toEqual({ light: { F2: 5 } });
  });
});
