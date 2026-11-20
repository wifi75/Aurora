import type { DigiMacro, DigiSettings } from '../api/client';
import { keyNames } from './macroLang';

/**
 * Importazione delle macro di fldigi (file `.mdf`, di base `macros.mdf` nella cartella `macros` di fldigi).
 *
 * Il formato (lo descrive il caricatore di fldigi, `MACROTEXT::loadMacros`):
 * - la prima riga e' `//fldigi macro definition file`; con la parola `extended` i numeri sono quelli veri, senza (file vecchi di fldigi) le macro dalla 10 in poi
 *   vanno spostate di due posti;
 * - le righe che cominciano con `//` sono commenti;
 * - una macro comincia con `/$ <numero> <etichetta>` (numero da 0 a 47: quattro file da 12 tasti) e il testo viene nelle righe seguenti, fino alla macro dopo;
 * - le righe di testo si attaccano una all'altra: un a-capo si scrive come `\n` (barra e n) alla fine della riga.
 *
 * Le 48 macro di fldigi hanno lo stesso ordine dei 48 tasti di Aurora (F1-F12, poi Ctrl, Shift e Ctrl-Shift): la macro numero `i` va sul tasto `i`.
 * Le etichette di tag `<...>` di fldigi si traducono in quelle di Aurora solo dove il significato e' lo stesso (vedi {@link TAGS}); le altre restano come sono e
 * si elencano a chi importa, perche' le sistemi a mano.
 */

const MAX_MACROS = 48;

export interface FldigiMacro {
  /** Il numero della macro nel file, da 0 a 47. */
  index: number;
  label: string;
  text: string;
}

export interface FldigiFile {
  /** `false` per i file del vecchio fldigi, dove le macro dalla 10 in poi erano spostate di due posti. */
  extended: boolean;
  macros: FldigiMacro[];
}

/** Cosa va in una macro importata: il tasto di Aurora, l'etichetta, il testo gia' tradotto. */
export interface ImportedMacro {
  name: string;
  label: string;
  text: string;
}

export interface ImportReport {
  macros: ImportedMacro[];
  /** I tag di fldigi senza un equivalente sicuro in Aurora, senza ripetizioni (restano nel testo cosi' come sono). */
  untranslated: string[];
  /** Le macro del file lasciate fuori perche' vuote. */
  skipped: number;
}

/** Il tag di fldigi (tutto maiuscolo, con le parentesi) e quello di Aurora, dove il significato e' lo stesso. */
const TAGS: Readonly<Record<string, string>> = {
  '<MYCALL>': '<MYCALL>',
  '<MYNAME>': '<MYNAME>',
  '<MYQTH>': '<MYQTH>',
  '<MYLOC>': '<MYWWLOC>',
  '<CALL>': '<CALL>',
  '<NAME>': '<NAME>',
  '<QTH>': '<QTH>',
  '<LOC>': '<WWLOC>',
  // in fldigi `<RST>` e' il rapporto che si manda (campo «Out») e `<MYRST>` quello che si e' ricevuto (campo «In»)
  '<RST>': '<RSTS>',
  '<MYRST>': '<RSTR>',
  '<TX>': '<TX>',
  '<RX>': '<RX>',
  '<CLRRX>': '<CLEARRXWINDOW>',
  '<CLRTX>': '<CLEARTXWINDOW>',
  '<CLRQSO>': '<CLEARQSO>',
  '<LOG>': '<SAVEQSO>',
  '<VER>': '<VER>',
  '<MODE>': '<MODE>',
  '<BAND>': '<BAND>',
  '<CNTR>': '<QSONR>',
};

/** Legge un file `.mdf` di fldigi; `null` se la prima riga non e' quella dei file di fldigi. */
export function parseFldigiFile(content: string): FldigiFile | null {
  const lines = content.replace(/^﻿/, '').split(/\r?\n/);
  const first = lines[0] ?? '';
  if (!first.startsWith('//fldigi macro definition file')) return null;
  const extended = first.includes('extended');
  const name: string[] = Array.from({ length: MAX_MACROS }, () => '');
  const text: string[] = Array.from({ length: MAX_MACROS }, () => '');
  let current = 0;
  for (const raw of lines.slice(1)) {
    if (raw === '') continue;
    if (raw.startsWith('//')) continue;
    if (raw.startsWith('/$')) {
      const space = raw.indexOf(' ', 3);
      if (space !== -1) {
        let n = Number.parseInt(raw.slice(3), 10);
        if (!Number.isFinite(n) || n < 0 || n > MAX_MACROS - 1) break;
        if (!extended && n > 9) n += 2;
        if (n > MAX_MACROS - 1) break;
        current = n;
        name[n] = raw.slice(space + 1);
      }
      continue;
    }
    let line = raw;
    const nl = line.lastIndexOf('\\n');
    if (nl !== -1) line = `${line.slice(0, nl)}\n`;
    text[current] = (text[current] ?? '') + line;
  }
  const macros: FldigiMacro[] = [];
  for (let i = 0; i < MAX_MACROS; i++) macros.push({ index: i, label: name[i] ?? '', text: text[i] ?? '' });
  return { extended, macros };
}

/** Traduce i tag di un testo di macro; restituisce il testo e i tag di fldigi che non si sono potuti tradurre. */
export function convertFldigiText(text: string): { text: string; untranslated: string[] } {
  const untranslated: string[] = [];
  const out = text.replace(/<[^<>\n]{1,60}>/g, (tag) => {
    const known = TAGS[tag.toUpperCase()];
    if (known !== undefined) return known;
    // un tag con parametro (`<WPM:25>`) o sconosciuto: resta, e si dice a chi importa
    if (!untranslated.includes(tag)) untranslated.push(tag);
    return tag;
  });
  return { text: out, untranslated };
}

/** Le macro del file, vuote escluse, sui tasti di Aurora e con i tag tradotti. */
export function buildImport(file: FldigiFile): ImportReport {
  const keys = keyNames();
  const macros: ImportedMacro[] = [];
  const untranslated: string[] = [];
  let skipped = 0;
  for (const m of file.macros) {
    if (m.label.trim() === '' && m.text.trim() === '') {
      skipped += 1;
      continue;
    }
    const key = keys[m.index];
    if (key === undefined) continue;
    const conv = convertFldigiText(m.text);
    for (const t of conv.untranslated) if (!untranslated.includes(t)) untranslated.push(t);
    macros.push({ name: key, label: m.label, text: conv.text });
  }
  return { macros, untranslated, skipped };
}

export type ImportHow = 'add' | 'replace';

/**
 * Le macro dopo l'importazione.
 * - `where`: `all` = macro per tutti i modi, `mode` = solo per il modo in uso;
 * - `how`: `add` = solo i tasti presenti nel file prendono le macro importate (le altre del gruppo restano), `replace` = il gruppo scelto diventa solo quello del file.
 * L'altro gruppo (quello non scelto) non si tocca.
 */
export function applyImport(
  s: Pick<DigiSettings, 'macros' | 'mode_macros' | 'mode'>,
  imported: readonly ImportedMacro[],
  where: 'all' | 'mode',
  how: ImportHow,
): Pick<DigiSettings, 'macros' | 'mode_macros'> {
  const entries: DigiMacro[] = imported.map((m) => ({ name: m.name, label: m.label, text: m.text, color: null }));
  const names = new Set(entries.map((e) => e.name));
  const merge = (current: readonly DigiMacro[]): DigiMacro[] => (how === 'replace' ? entries : [...current.filter((m) => !names.has(m.name)), ...entries]);
  if (where === 'all') return { macros: merge(s.macros), mode_macros: { ...s.mode_macros } };
  return { macros: [...s.macros], mode_macros: { ...s.mode_macros, [s.mode]: merge(s.mode_macros[s.mode] ?? []) } };
}
