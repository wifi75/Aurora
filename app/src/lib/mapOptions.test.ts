import { describe, expect, it } from 'vitest';
import { readMapOptions, saveMapOptions } from './mapOptions';

describe('map options', () => {
  it('start with the real background and the night on', () => {
    expect(readMapOptions({ getItem: () => null })).toEqual({ tiles: true, night: true });
  });

  it('remember what was chosen', () => {
    const store: Record<string, string> = {};
    saveMapOptions({ tiles: false, night: true }, { setItem: (k, v) => { store[k] = v; } });
    expect(readMapOptions({ getItem: (k) => store[k] ?? null })).toEqual({ tiles: false, night: true });
  });

  it('ignore a damaged or unreadable value and never throw', () => {
    expect(readMapOptions({ getItem: () => '{non json' })).toEqual({ tiles: true, night: true });
    expect(readMapOptions({ getItem: () => '{"tiles":"si","night":false}' })).toEqual({ tiles: true, night: false });
    expect(readMapOptions({ getItem: () => { throw new Error('no'); } })).toEqual({ tiles: true, night: true });
    expect(() => saveMapOptions({ tiles: true, night: true }, { setItem: () => { throw new Error('no'); } })).not.toThrow();
  });
});
