import { beforeEach, describe, expect, it } from 'vitest';
import { defaultPrefs, formatDistance, readPrefs, savePrefs, usePrefs } from './prefs';

function memory(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return { data, getItem: (k: string) => data[k] ?? null, setItem: (k: string, v: string) => { data[k] = v; } };
}
const broken = { getItem: () => { throw new Error('no'); }, setItem: () => { throw new Error('no'); } };

describe('display preferences', () => {
  beforeEach(() => localStorage.clear());

  it('start in kilometres with country, flag, distance and DT shown', () => {
    expect(defaultPrefs()).toEqual({ distanceUnit: 'km', showCountry: true, showFlag: true, showDistance: true, showDt: true });
    expect(readPrefs(memory())).toEqual(defaultPrefs());
    expect(readPrefs(null)).toEqual(defaultPrefs());
  });

  it('are remembered and sanitised', () => {
    const store = memory();
    savePrefs(store, { ...defaultPrefs(), distanceUnit: 'mi', showFlag: false });
    expect(readPrefs(store)).toEqual({ ...defaultPrefs(), distanceUnit: 'mi', showFlag: false });
    expect(readPrefs(memory({ 'aurora-prefs': '{"distanceUnit":"leghe","showCountry":"no","showDt":false}' }))).toEqual({ ...defaultPrefs(), showDt: false });
    expect(readPrefs(memory({ 'aurora-prefs': 'non json' }))).toEqual(defaultPrefs());
    expect(readPrefs(broken)).toEqual(defaultPrefs());
    expect(() => savePrefs(broken, defaultPrefs())).not.toThrow();
  });

  it('the shared store updates and persists at once', () => {
    usePrefs.getState().set({ distanceUnit: 'mi' });
    expect(usePrefs.getState().prefs.distanceUnit).toBe('mi');
    expect(JSON.parse(localStorage.getItem('aurora-prefs') ?? '{}').distanceUnit).toBe('mi');
    usePrefs.getState().reset();
    expect(usePrefs.getState().prefs).toEqual(defaultPrefs());
  });
});

describe('formatDistance', () => {
  it('writes kilometres or miles', () => {
    expect(formatDistance(1000, 'km')).toBe('1000 km');
    expect(formatDistance(1000, 'mi')).toBe('621 mi');
    expect(formatDistance(0, 'km')).toBe('0 km');
  });
});
