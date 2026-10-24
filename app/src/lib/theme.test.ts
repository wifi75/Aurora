import { describe, expect, it } from 'vitest';
import { applyTheme, readTheme, saveTheme } from './theme';

function memory(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return {
    data,
    getItem: (k: string) => data[k] ?? null,
    setItem: (k: string, v: string) => {
      data[k] = v;
    },
  };
}

const broken = {
  getItem: () => {
    throw new Error('archivio non disponibile');
  },
  setItem: () => {
    throw new Error('archivio non disponibile');
  },
};

describe('theme', () => {
  it('defaults to the dark theme "Notte radio"', () => {
    expect(readTheme(memory())).toBe('dark');
    expect(readTheme(null)).toBe('dark');
  });

  it('reads a saved light choice and ignores garbage', () => {
    expect(readTheme(memory({ 'aurora-theme': 'light' }))).toBe('light');
    expect(readTheme(memory({ 'aurora-theme': 'arcobaleno' }))).toBe('dark');
  });

  it('falls back to dark when the storage fails', () => {
    expect(readTheme(broken)).toBe('dark');
  });

  it('saves the choice and survives a failing storage', () => {
    const m = memory();
    saveTheme(m, 'light');
    expect(m.data['aurora-theme']).toBe('light');
    expect(() => saveTheme(broken, 'dark')).not.toThrow();
    expect(() => saveTheme(null, 'dark')).not.toThrow();
  });

  it('applies the theme on the root element', () => {
    const root = document.createElement('html');
    applyTheme(root, 'light');
    expect(root.dataset.theme).toBe('light');
    applyTheme(root, 'dark');
    expect(root.dataset.theme).toBe('dark');
  });
});
