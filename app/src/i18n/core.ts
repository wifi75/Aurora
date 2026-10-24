/** Le sei lingue di Aurora; l'ordine e' quello delle colonne di ogni tabella di testi. */
export const LANGS = ['it', 'en', 'de', 'fr', 'es', 'pt'] as const;
export type Lang = (typeof LANGS)[number];

export const LANG_NAMES: Record<Lang, string> = {
  it: 'Italiano',
  en: 'English',
  de: 'Deutsch',
  fr: 'Français',
  es: 'Español',
  pt: 'Português',
};

/** Un testo nelle sei lingue: [it, en, de, fr, es, pt]. I segnaposto sono `{nome}`. */
export type Entry = readonly [string, string, string, string, string, string];

export type Table = Record<string, Entry>;

export const DEFAULT_LANG: Lang = 'it';

export function isLang(value: unknown): value is Lang {
  return typeof value === 'string' && (LANGS as readonly string[]).includes(value);
}

/** Sostituisce `{nome}` con il valore; un segnaposto senza valore resta com'e'. */
export function fill(text: string, vars?: Record<string, string | number>): string {
  if (!vars) return text;
  return text.replace(/\{(\w+)\}/g, (whole, name: string) => (name in vars ? String(vars[name]) : whole));
}

export function pick(entry: Entry, lang: Lang): string {
  return entry[LANGS.indexOf(lang)] ?? entry[0];
}
