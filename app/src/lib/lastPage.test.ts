import { describe, expect, it } from 'vitest';
import { lastPageToRestore, rememberRoute, rememberTab, rememberedTab } from './lastPage';

function memory() {
  const data = new Map<string, string>();
  return { getItem: (k: string) => data.get(k) ?? null, setItem: (k: string, v: string) => void data.set(k, v) };
}

describe('lastPage', () => {
  it('starts on the page that was open last, only when no address was given', () => {
    const s = memory();
    rememberRoute(s, { name: 'map' });
    expect(lastPageToRestore(s, '')).toBe('#/mappa');
    expect(lastPageToRestore(s, '#/')).toBe('#/mappa');
    expect(lastPageToRestore(s, '#/storico')).toBeNull();
    expect(lastPageToRestore(s, '#/pair/12345678')).toBeNull();
  });

  it('goes back to the station when that was the last page', () => {
    const s = memory();
    rememberRoute(s, { name: 'map' });
    rememberRoute(s, { name: 'live' });
    expect(lastPageToRestore(s, '')).toBeNull();
  });

  it('does not remember the pairing step', () => {
    const s = memory();
    rememberRoute(s, { name: 'history' });
    rememberRoute(s, { name: 'pair', code: '12345678' });
    expect(lastPageToRestore(s, '')).toBe('#/storico');
  });

  it('remembers the settings tab, from the address or from a click', () => {
    const s = memory();
    expect(rememberedTab(s)).toBeUndefined();
    rememberRoute(s, { name: 'settings', tab: 'rig' });
    expect(rememberedTab(s)).toBe('rig');
    rememberTab(s, 'audio');
    expect(rememberedTab(s)).toBe('audio');
    rememberRoute(s, { name: 'settings' });
    expect(rememberedTab(s)).toBe('audio');
    expect(lastPageToRestore(s, '')).toBe('#/impostazioni');
  });

  it('ignores a saved value that is not a page and works without storage', () => {
    const s = memory();
    s.setItem('aurora-last-page', '#/qualcosa');
    expect(lastPageToRestore(s, '')).toBeNull();
    expect(lastPageToRestore(null, '')).toBeNull();
    rememberRoute(null, { name: 'map' });
    expect(rememberedTab(null)).toBeUndefined();
  });
});
