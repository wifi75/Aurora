import { beforeEach, describe, expect, it } from 'vitest';
import { DEFAULT_LANG, LANGS, LANG_NAMES, fill, isLang, pick, type Entry } from './core';
import { readLang, saveLang } from './lang';
import { STRINGS, translate, type Key } from './index';

function memory(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return { data, getItem: (k: string) => data[k] ?? null, setItem: (k: string, v: string) => { data[k] = v; } };
}
const broken = { getItem: () => { throw new Error('no'); }, setItem: () => { throw new Error('no'); } };

const placeholders = (text: string) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(',');

describe('languages', () => {
  it('are the six of the app, Italian first', () => {
    expect(LANGS).toEqual(['it', 'en', 'de', 'fr', 'es', 'pt']);
    expect(DEFAULT_LANG).toBe('it');
    for (const l of LANGS) expect(LANG_NAMES[l]).toBeTruthy();
  });

  it('recognises only supported codes', () => {
    expect(isLang('de')).toBe(true);
    expect(isLang('ru')).toBe(false);
    expect(isLang(7)).toBe(false);
  });
});

describe('the texts', () => {
  const entries = Object.entries(STRINGS) as Array<[string, Entry]>;

  it('exist and are not empty in every language', () => {
    expect(entries.length).toBeGreaterThan(50);
    for (const [key, entry] of entries) {
      expect(entry, key).toHaveLength(6);
      for (const [i, text] of entry.entries()) expect(text.trim(), `${key} [${LANGS[i]}]`).not.toBe('');
    }
  });

  it('use the same placeholders in every language', () => {
    for (const [key, entry] of entries) {
      const expected = placeholders(entry[0]);
      for (const [i, text] of entry.entries()) expect(placeholders(text), `${key} [${LANGS[i]}]`).toBe(expected);
    }
  });

  it('are actually translated: the English text differs from the Italian one for most keys', () => {
    const same = entries.filter(([, e]) => e[0] === e[1]).length;
    expect(same / entries.length).toBeLessThan(0.35);
  });
});

describe('translate', () => {
  it('returns the text of the requested language and fills placeholders', () => {
    const key = 'nav.station' as Key;
    const entry = STRINGS[key] as Entry;
    expect(translate('it', key)).toBe(pick(entry, 'it'));
    expect(translate('en', key)).toBe(pick(entry, 'en'));
    expect(fill('Ciao {nome}, {n} volte', { nome: 'Ada', n: 3 })).toBe('Ciao Ada, 3 volte');
    expect(fill('Ciao {nome}', {})).toBe('Ciao {nome}');
  });

  it('falls back to the key text for an unknown key instead of failing', () => {
    expect(translate('en', 'chiave.che.non.esiste' as Key)).toBe('chiave.che.non.esiste');
  });
});

describe('the remembered language', () => {
  beforeEach(() => localStorage.clear());

  it('is Italian by default', () => {
    expect(readLang(memory())).toBe('it');
    expect(readLang(null)).toBe('it');
  });

  it('is remembered and ignores garbage or a broken storage', () => {
    const store = memory();
    saveLang(store, 'fr');
    expect(readLang(store)).toBe('fr');
    expect(readLang(memory({ 'aurora-lang': 'klingon' }))).toBe('it');
    expect(readLang(broken)).toBe('it');
    expect(() => saveLang(broken, 'de')).not.toThrow();
  });
});
