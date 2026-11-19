import { describe, expect, it } from 'vitest';
import { clearLayouts, isLayoutKey } from './resetLayouts';

describe('resetLayouts', () => {
  it('riconosce le voci di disposizione e non tocca le altre scelte', () => {
    for (const k of ['aurora-board', 'aurora-bottom', 'aurora-layout', 'aurora-finestre', 'aurora.digi.winw', 'aurora.band.cols', 'aurora.digi.radar.cols', 'aurora.sat.layout', 'aurora.xrx.2']) {
      expect(isLayoutKey(k), k).toBe(true);
    }
    for (const k of ['aurora-lang', 'aurora-theme', 'aurora-appearance', 'aurora-ui-scale', 'aurora.digi.campi', 'aurora-filters', 'aurora-sounds', 'altro-board']) {
      expect(isLayoutKey(k), k).toBe(false);
    }
  });

  it('toglie solo le disposizioni dal browser', () => {
    const data = new Map<string, string>([
      ['aurora-board', '{}'],
      ['aurora.cluster.cols', '[]'],
      ['aurora-theme', 'dark'],
      ['aurora.digi.campi', '{"nome":"x"}'],
    ]);
    const store = {
      get length() {
        return data.size;
      },
      key: (i: number) => [...data.keys()][i] ?? null,
      removeItem: (k: string) => void data.delete(k),
    };
    expect(clearLayouts(store)).toBe(2);
    expect([...data.keys()].sort()).toEqual(['aurora-theme', 'aurora.digi.campi']);
  });
});
