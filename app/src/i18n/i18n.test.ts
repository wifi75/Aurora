import { beforeEach, describe, expect, it } from 'vitest';
import { DEFAULT_LANG, LANGS, LANG_NAMES, TABLE_LANGS, detectLang, fill, isLang, langFromLocale, localeOf, pick, textKey, type Entry } from './core';
import { firstLaunchLang, readLang, saveLang } from './lang';
import { GUIDE } from './guide';
import { NOVITA } from '../lib/novita';
import { STRINGS, keyForSystem, textOf, translate, type Key } from './index';
import { NL_TEXTS } from './nl/texts';
import { NL_UI } from './nl/ui';

function memory(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return { data, getItem: (k: string) => data[k] ?? null, setItem: (k: string, v: string) => { data[k] = v; } };
}
const broken = { getItem: () => { throw new Error('no'); }, setItem: () => { throw new Error('no'); } };

const placeholders = (text: string) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(',');

describe('languages', () => {
  it('are the six of the tables plus Dutch, Italian first', () => {
    expect(TABLE_LANGS).toEqual(['it', 'en', 'de', 'fr', 'es', 'pt']);
    expect(LANGS).toEqual(['it', 'en', 'de', 'fr', 'es', 'pt', 'nl']);
    expect(LANG_NAMES.nl).toBe('Nederlands');
    expect(DEFAULT_LANG).toBe('it');
    for (const l of LANGS) expect(LANG_NAMES[l]).toBeTruthy();
  });

  it('recognises only supported codes', () => {
    expect(isLang('de')).toBe(true);
    expect(isLang('ru')).toBe(false);
    expect(isLang(7)).toBe(false);
    expect(isLang('nl')).toBe(true);
  });

  it('maps the system or browser language, Belgian Dutch included, to the interface language', () => {
    expect(langFromLocale('nl')).toBe('nl');
    expect(langFromLocale('nl-BE')).toBe('nl');
    expect(langFromLocale('NL_nl')).toBe('nl');
    expect(langFromLocale('de-AT')).toBe('de');
    expect(langFromLocale('ru-RU')).toBeNull();
    expect(langFromLocale(undefined)).toBeNull();
    expect(detectLang(['ru-RU', 'nl-BE', 'en'])).toBe('nl');
    expect(detectLang(['ja'])).toBeNull();
    expect(detectLang(null)).toBeNull();
  });

  it('formats dates and numbers for Dutch with nl-NL and leaves the other languages as they were', () => {
    expect(localeOf('nl')).toBe('nl-NL');
    expect(localeOf('en')).toBe('en');
    expect(localeOf('xx')).toBe('xx');
    expect(new Date(Date.UTC(2026, 10, 4, 12)).toLocaleDateString(localeOf('nl'), { month: 'long', timeZone: 'UTC' })).toBe('november');
    expect((1234567).toLocaleString(localeOf('nl'))).toBe('1.234.567');
  });
});

describe('the texts', () => {
  const entries = Object.entries(STRINGS) as Array<[string, Entry]>;

  it('exist and are not empty in every language', () => {
    expect(entries.length).toBeGreaterThan(50);
    for (const [key, entry] of entries) {
      expect(entry, key).toHaveLength(6);
      for (const [i, text] of entry.entries()) expect(text.trim(), `${key} [${TABLE_LANGS[i]}]`).not.toBe('');
    }
  });

  it('use the same placeholders in every language', () => {
    for (const [key, entry] of entries) {
      const expected = placeholders(entry[0]);
      for (const [i, text] of entry.entries()) expect(placeholders(text), `${key} [${TABLE_LANGS[i]}]`).toBe(expected);
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

describe('texts that name Windows', () => {
  it('have a variant for the other systems, chosen from the system the engine runs on', () => {
    expect(translate('en', 'aud.vol.off')).toBe('Windows volume not available for this device.');
    expect(translate('en', 'aud.vol.off', undefined, true)).toBe('The volume of this device is set from the system audio mixer.');
    expect(translate('it', 'aud.vol.out', undefined, true)).toBe('Livello di trasmissione (Pwr)');
    expect(translate('en', 'rxl.volume.value', { pct: 40, device: 'USB' }, true)).toBe('40 % of the system recording volume (USB)');
    // una chiave senza variante resta quella di sempre, su ogni sistema
    expect(keyForSystem('nav.station' as Key, true)).toBe('nav.station');
    expect(translate('it', 'nav.station' as Key, undefined, true)).toBe(translate('it', 'nav.station' as Key));
  });

  it('every variant belongs to an existing text, keeps its placeholders and never names Windows', () => {
    const variants = (Object.keys(STRINGS) as Key[]).filter((k) => k.endsWith('.sys'));
    expect(variants.length).toBeGreaterThan(10);
    for (const key of variants) {
      const base = key.slice(0, -'.sys'.length) as Key;
      expect(STRINGS[base], `${key}: manca il testo di base`).toBeDefined();
      for (const lang of TABLE_LANGS) {
        expect(placeholders(textOf(lang, key) ?? ''), `${key} ${lang}`).toBe(placeholders(textOf(lang, base) ?? ''));
        expect(textOf(lang, key), `${key} ${lang}`).not.toMatch(/Windows/);
      }
      expect(textOf('nl', key), `${key} nl`).not.toMatch(/Windows/);
    }
  });

  it('no text shown on the Audio page outside Windows names it', () => {
    for (const key of ['aud.vol.in', 'aud.vol.out', 'tip.aud.vol.in', 'tip.aud.vol.out', 'aud.vol.off', 'tip.aud.input', 'ens.audio.help'] as Key[]) {
      for (const lang of [...TABLE_LANGS, 'nl'] as const) expect(translate(lang, key, undefined, true), `${key} ${lang}`).not.toMatch(/Windows/);
    }
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

describe('Dutch (separate dictionary, English fallback)', () => {
  const placeholdersAndMarks = (text: string) => [...text.matchAll(/\{\w+\}|\[\[[\w.]+\]\]/g)].map((m) => m[0]).sort().join('|');
  const anon: Entry[] = [
    ...GUIDE.flatMap((c) => [c.title, ...c.sections.flatMap((s) => [s.title, s.text, s.tip, ...(s.steps ?? []), ...(s.fix ?? [])].filter((e): e is Entry => !!e))]),
    ...NOVITA.flatMap((n) => [n.title, n.text]),
  ];

  it('has no orphan keys: every Dutch key exists in the base tables', () => {
    for (const key of Object.keys(NL_UI)) expect(key in STRINGS, `chiave olandese senza testo base: ${key}`).toBe(true);
    const known = new Set(anon.map(textKey));
    for (const key of Object.keys(NL_TEXTS)) expect(known.has(key), `voce olandese di Guida/Novita' senza testo base: ${key}`).toBe(true);
  });

  it('keeps placeholders and [[key]] markers of the English text', () => {
    for (const [key, text] of Object.entries(NL_UI)) expect(placeholdersAndMarks(text), key).toBe(placeholdersAndMarks((STRINGS[key as Key] as Entry)[1]));
    for (const entry of anon) {
      const nl = NL_TEXTS[textKey(entry)];
      if (nl !== undefined) expect(placeholdersAndMarks(nl), entry[1]).toBe(placeholdersAndMarks(entry[1]));
    }
  });

  it('gives each Guida/Novita text its own stable key', () => {
    const byKey = new Map<string, string>();
    for (const e of anon) {
      const k = textKey(e);
      expect(byKey.get(k) ?? e[1], k).toBe(e[1]);
      byKey.set(k, e[1]);
    }
    expect(textKey(anon[0]!)).toMatch(/^t\.[0-9a-z]+$/);
  });

  it('translates with the Dutch dictionary', () => {
    const key = 'nav.station' as Key;
    expect(NL_UI[key]).toBeTruthy();
    expect(translate('nl', key)).toBe(NL_UI[key]);
    expect(translate('nl', key)).not.toBe(translate('en', key) + '#');
    const entry = GUIDE[0]!.title;
    expect(pick(entry, 'nl')).toBe(NL_TEXTS[textKey(entry)] ?? entry[1]);
  });

  it('falls back to English when a Dutch text is missing', () => {
    const key = 'nav.station' as Key;
    const saved = NL_UI[key];
    delete NL_UI[key];
    try {
      expect(translate('nl', key)).toBe(pick(STRINGS[key] as Entry, 'en'));
      expect(textOf('nl', key)).toBe((STRINGS[key] as Entry)[1]);
    } finally {
      NL_UI[key] = saved!;
    }
    const unknown: Entry = ['Solo italiano', 'English only', 'a', 'b', 'c', 'd'];
    expect(pick(unknown, 'nl')).toBe('English only');
    expect(pick(unknown, 'it')).toBe('Solo italiano');
  });

  it('is complete for the whole interface in this first translation', () => {
    expect(Object.keys(NL_UI).length).toBe(Object.keys(STRINGS).length);
  });
});

describe('first launch language', () => {
  it('follows the system language, Dutch included, only while no language was chosen', () => {
    expect(firstLaunchLang(memory(), ['nl-BE', 'en'])).toBe('nl');
    expect(firstLaunchLang(memory({ 'aurora-lang': 'de' }), ['nl-NL'])).toBeNull();
    expect(firstLaunchLang(memory(), ['ja-JP'])).toBeNull();
    expect(firstLaunchLang(broken, ['nl'])).toBeNull();
    expect(firstLaunchLang(null, ['nl'])).toBe('nl');
    // installazione gia' avviata senza lingua scelta: resta l'italiano di sempre
    expect(firstLaunchLang(memory({ 'aurora-first-run-done': '1' }), ['nl-NL'])).toBeNull();
  });
});
