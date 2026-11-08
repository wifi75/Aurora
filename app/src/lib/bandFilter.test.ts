import { describe, expect, it } from 'vitest';
import { allBands, defaultBands, readBands, saveBands, toggleBand, visibleBands } from './bandFilter';

function memory(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return { data, getItem: (k: string) => data[k] ?? null, setItem: (k: string, v: string) => { data[k] = v; } };
}
const broken = { getItem: () => { throw new Error('no'); }, setItem: () => { throw new Error('no'); } };

describe('bandFilter', () => {
  it('shows every band by default', () => {
    expect(readBands(memory())).toEqual(defaultBands());
    expect(readBands(null)).toEqual(defaultBands());
    expect(defaultBands()).toHaveLength(12);
    expect(allBands()).toHaveLength(25);
  });

  it('remembers a chosen selection in band order', () => {
    const store = memory();
    saveBands(store, ['40m', '20m', '15m']);
    expect(readBands(store)).toEqual(['40m', '20m', '15m']);
  });

  it('drops unknown names and never ends up empty', () => {
    expect(readBands(memory({ 'aurora-bands': '["20m","13m",7]' }))).toEqual(['20m']);
    expect(readBands(memory({ 'aurora-bands': '[]' }))).toEqual(defaultBands());
    expect(readBands(memory({ 'aurora-bands': '["13m"]' }))).toEqual(defaultBands());
    expect(readBands(memory({ 'aurora-bands': 'non json' }))).toEqual(defaultBands());
    expect(readBands(broken)).toEqual(defaultBands());
    expect(() => saveBands(broken, ['20m'])).not.toThrow();
  });

  it('toggles a band, keeping the order and at least one band', () => {
    expect(toggleBand(['20m', '40m'], '15m')).toEqual(['40m', '20m', '15m']);
    expect(toggleBand(['40m', '20m', '15m'], '20m')).toEqual(['40m', '15m']);
    expect(toggleBand(['20m'], '20m')).toEqual(['20m']);
  });

  it('always keeps the current band visible', () => {
    expect(visibleBands(['40m', '20m'], '15m')).toEqual(['40m', '20m', '15m']);
    expect(visibleBands(['40m', '20m'], '20m')).toEqual(['40m', '20m']);
    expect(visibleBands(['40m'], null)).toEqual(['40m']);
  });
});
