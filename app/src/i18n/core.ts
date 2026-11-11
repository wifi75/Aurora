import { NL_TEXTS } from './nl/texts';

/** Le sei lingue delle tabelle di testi; l'ordine e' quello delle colonne di ogni tabella. */
export const TABLE_LANGS = ['it', 'en', 'de', 'fr', 'es', 'pt'] as const;
export type TableLang = (typeof TABLE_LANGS)[number];

/**
 * Le lingue che l'utente puo' scegliere: le sei delle tabelle + l'olandese, che sta in un dizionario a parte (`i18n/nl/`)
 * e, dove manca una voce, ricade sull'inglese.
 */
export const LANGS = [...TABLE_LANGS, 'nl'] as const;
export type Lang = (typeof LANGS)[number];

export const LANG_NAMES: Record<Lang, string> = {
  it: 'Italiano',
  en: 'English',
  de: 'Deutsch',
  fr: 'Français',
  es: 'Español',
  pt: 'Português',
  nl: 'Nederlands',
};

/** Impostazione regionale per date e numeri (`toLocaleString`, `Intl`). */
export const LOCALES: Record<Lang, string> = { it: 'it', en: 'en', de: 'de', fr: 'fr', es: 'es', pt: 'pt', nl: 'nl-NL' };

export function localeOf(lang: string): string {
  return isLang(lang) ? LOCALES[lang] : lang;
}

/** Un testo nelle sei lingue: [it, en, de, fr, es, pt]. I segnaposto sono `{nome}`. */
export type Entry = readonly [string, string, string, string, string, string];

export type Table = Record<string, Entry>;

export const DEFAULT_LANG: Lang = 'it';

export function isLang(value: unknown): value is Lang {
  return typeof value === 'string' && (LANGS as readonly string[]).includes(value);
}

/** La lingua dell'interfaccia che corrisponde a un codice del sistema o del browser (`nl-BE` -> `nl`); `null` se non e' tra le nostre. */
export function langFromLocale(tag: string | null | undefined): Lang | null {
  const base = (tag ?? '').trim().toLowerCase().split(/[-_]/)[0] ?? '';
  return isLang(base) ? base : null;
}

/** La prima lingua dell'elenco preferito (`navigator.languages`) che Aurora conosce. */
export function detectLang(tags: readonly string[] | null | undefined): Lang | null {
  for (const tag of tags ?? []) {
    const lang = langFromLocale(tag);
    if (lang) return lang;
  }
  return null;
}

/** Sostituisce `{nome}` con il valore; un segnaposto senza valore resta com'e'. */
export function fill(text: string, vars?: Record<string, string | number>): string {
  if (!vars) return text;
  return text.replace(/\{(\w+)\}/g, (whole, name: string) => (name in vars ? String(vars[name]) : whole));
}

/**
 * Chiave stabile di un testo senza nome (Guida, Novita'): `t.` + impronta (cyrb53) del testo inglese.
 * Se l'inglese cambia, la chiave cambia: l'olandese ricade sull'inglese finche' non viene riesportato e rivisto.
 */
export function textKey(entry: Entry): string {
  let h1 = 0xdeadbeef;
  let h2 = 0x41c6ce57;
  const s = entry[1];
  for (let i = 0; i < s.length; i++) {
    const ch = s.charCodeAt(i);
    h1 = Math.imul(h1 ^ ch, 2654435761);
    h2 = Math.imul(h2 ^ ch, 1597334677);
  }
  h1 = Math.imul(h1 ^ (h1 >>> 16), 2246822507) ^ Math.imul(h2 ^ (h2 >>> 13), 3266489909);
  h2 = Math.imul(h2 ^ (h2 >>> 16), 2246822507) ^ Math.imul(h1 ^ (h1 >>> 13), 3266489909);
  return `t.${(4294967296 * (2097151 & h2) + (h1 >>> 0)).toString(36)}`;
}

/** Il testo nella lingua scelta; in olandese quello del dizionario a parte, o l'inglese se manca. */
export function pick(entry: Entry, lang: Lang): string {
  if (lang === 'nl') return NL_TEXTS[textKey(entry)] ?? entry[1];
  return entry[TABLE_LANGS.indexOf(lang)] ?? entry[0];
}
