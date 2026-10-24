import { describe, expect, it } from 'vitest';
import { readSize, saveSize } from './waterfallSize';

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

describe('waterfallSize', () => {
  it('defaults to the large waterfall', () => {
    expect(readSize(memory())).toBe('large');
    expect(readSize(null)).toBe('large');
  });

  it('reads a saved size and ignores garbage', () => {
    expect(readSize(memory({ 'aurora-waterfall': 'small' }))).toBe('small');
    expect(readSize(memory({ 'aurora-waterfall': 'hidden' }))).toBe('hidden');
    expect(readSize(memory({ 'aurora-waterfall': 'enorme' }))).toBe('large');
  });

  it('falls back to large when the storage fails', () => {
    expect(readSize(broken)).toBe('large');
  });

  it('saves the choice and survives a failing storage', () => {
    const m = memory();
    saveSize(m, 'small');
    expect(m.data['aurora-waterfall']).toBe('small');
    expect(() => saveSize(broken, 'hidden')).not.toThrow();
    expect(() => saveSize(null, 'large')).not.toThrow();
  });
});
