// Carica i testi di Aurora (tabelle in sei lingue, Guida, Novita') e il dizionario russo, senza avviare l'app.
// Serve a export-ru.mjs e importa-ru.mjs; i campi CSV si leggono e scrivono con le funzioni di nl-modello.mjs. Richiede `app/node_modules`.
import { createRequire } from 'node:module';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { APP, RADICE } from './nl-modello.mjs';

export { csvCampo, leggiCsv, scriviDizionario } from './nl-modello.mjs';
export const FILE_UI = join(APP, 'src', 'i18n', 'ru', 'ui.ts');
export const FILE_TESTI = join(APP, 'src', 'i18n', 'ru', 'texts.ts');
export const FILE_CSV = join(RADICE, 'docs', 'traduzione-russa.csv');

/** Delle Novita' si traducono in russo solo le piu' recenti (quelle in testa a `NOVITA`); le altre restano in inglese. */
export const NOVITA_RU = 10;

export async function caricaModello(novitaMax = NOVITA_RU) {
  const require = createRequire(join(APP, 'package.json'));
  const { rolldown } = require('rolldown');
  const tmp = mkdtempSync(join(tmpdir(), 'aurora-ru-'));
  try {
    const entry = join(tmp, 'entry.ts');
    const src = (p) => JSON.stringify(join(APP, 'src', p).replaceAll('\\', '/'));
    writeFileSync(
      entry,
      [
        `export { STRINGS } from ${src('i18n/index.ts')};`,
        `export { textKey } from ${src('i18n/core.ts')};`,
        `export { GUIDE } from ${src('i18n/guide.ts')};`,
        `export { NOVITA } from ${src('lib/novita.ts')};`,
        `export { MACRO_COMMANDS, MACRO_EVENTS } from ${src('lib/macroCommands.ts')};`,
        `export { RU_UI } from ${src('i18n/ru/ui.ts')};`,
        `export { RU_TEXTS } from ${src('i18n/ru/texts.ts')};`,
        '',
      ].join('\n'),
    );
    const build = await rolldown({ input: entry, platform: 'node', logLevel: 'silent' });
    await build.write({ file: join(tmp, 'out.mjs'), format: 'esm' });
    const m = await import(pathToFileURL(join(tmp, 'out.mjs')).href);

    const ui = Object.entries(m.STRINGS).map(([key, e]) => ({ key, en: e[1] }));
    const testi = new Map(); // chiave -> { key, en, area }
    const aggiungi = (entry, area) => {
      if (!entry) return;
      const k = m.textKey(entry);
      if (!testi.has(k)) testi.set(k, { key: k, en: entry[1], area });
    };
    for (const c of m.GUIDE) {
      aggiungi(c.title, 'guide');
      for (const s of c.sections) {
        aggiungi(s.title, 'guide');
        aggiungi(s.text, 'guide');
        aggiungi(s.tip, 'guide');
        for (const e of s.steps ?? []) aggiungi(e, 'guide');
        for (const e of s.fix ?? []) aggiungi(e, 'guide');
      }
    }
    for (const n of m.NOVITA.slice(0, novitaMax)) {
      aggiungi(n.title, 'news');
      aggiungi(n.text, 'news');
    }
    // l'elenco dei comandi delle macro (editor e Guida alle macro)
    for (const c of m.MACRO_COMMANDS) {
      aggiungi(c.desc, 'macro');
      aggiungi(c.long, 'macro');
    }
    for (const e of m.MACRO_EVENTS) aggiungi(e.desc, 'macro');
    return { ui, testi: [...testi.values()], ruUi: m.RU_UI, ruTesti: m.RU_TEXTS };
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}
