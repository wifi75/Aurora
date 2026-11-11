// Carica i testi di Aurora (tabelle in sei lingue, Guida, Novita') e il dizionario olandese, senza avviare l'app.
// Serve a export-nl.mjs e importa-nl.mjs. Richiede `app/node_modules` (rolldown arriva con Vite).
import { createRequire } from 'node:module';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

export const RADICE = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export const APP = join(RADICE, 'app');
export const FILE_UI = join(APP, 'src', 'i18n', 'nl', 'ui.ts');
export const FILE_TESTI = join(APP, 'src', 'i18n', 'nl', 'texts.ts');
export const FILE_CSV = join(RADICE, 'docs', 'traduzione-olandese.csv');

export async function caricaModello() {
  const require = createRequire(join(APP, 'package.json'));
  const { rolldown } = require('rolldown');
  const tmp = mkdtempSync(join(tmpdir(), 'aurora-nl-'));
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
        `export { NL_UI } from ${src('i18n/nl/ui.ts')};`,
        `export { NL_TEXTS } from ${src('i18n/nl/texts.ts')};`,
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
    for (const n of m.NOVITA) {
      aggiungi(n.title, 'news');
      aggiungi(n.text, 'news');
    }
    return { ui, testi: [...testi.values()], nlUi: m.NL_UI, nlTesti: m.NL_TEXTS };
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

/** Campo CSV con le virgolette doppie quando serve (il file e' UTF-8 con BOM e righe CRLF: si apre bene in Excel). */
export function csvCampo(v) {
  const s = String(v ?? '');
  return /[",\r\n]/.test(s) || /^\s|\s$/.test(s) ? `"${s.replaceAll('"', '""')}"` : s;
}

export function leggiCsv(testo) {
  const righe = [];
  let riga = [];
  let campo = '';
  let tra = false;
  const t = testo.replace(/^﻿/, '');
  for (let i = 0; i < t.length; i++) {
    const c = t[i];
    if (tra) {
      if (c === '"') {
        if (t[i + 1] === '"') {
          campo += '"';
          i++;
        } else tra = false;
      } else campo += c;
    } else if (c === '"') tra = true;
    else if (c === ',') {
      riga.push(campo);
      campo = '';
    } else if (c === '\n' || c === '\r') {
      if (c === '\r' && t[i + 1] === '\n') i++;
      riga.push(campo);
      campo = '';
      if (riga.some((x) => x !== '')) righe.push(riga);
      riga = [];
    } else campo += c;
  }
  riga.push(campo);
  if (riga.some((x) => x !== '')) righe.push(riga);
  return righe;
}

/** Il file TypeScript del dizionario, con le chiavi in ordine stabile. */
export function scriviDizionario(nome, doc, mappa) {
  const righe = Object.keys(mappa)
    .sort()
    .map((k) => `  ${JSON.stringify(k)}: ${JSON.stringify(mappa[k])},`);
  return `/** ${doc} */\nexport const ${nome}: Record<string, string> = {\n${righe.join('\n')}\n};\n`;
}
