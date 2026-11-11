// Rilegge docs/traduzione-olandese.csv (o il file dato) e riscrive il dizionario olandese (app/src/i18n/nl/ui.ts e texts.ts).
// Le voci con la colonna «Dutch» vuota non entrano nel dizionario (in app ricadono sull'inglese). Le chiavi che non esistono piu' si scartano.
// Controlla che i segnaposto {nome} e i riferimenti [[chiave]] dell'inglese ci siano anche nell'olandese: se no, avvisa e salta la voce.
// Uso: node scripts/importa-nl.mjs [file.csv]
import { readFileSync, writeFileSync } from 'node:fs';
import { FILE_CSV, FILE_TESTI, FILE_UI, caricaModello, leggiCsv, scriviDizionario } from './nl-modello.mjs';

const file = process.argv[2] ?? FILE_CSV;
const { ui, testi } = await caricaModello();
const inglese = new Map([...ui, ...testi].map((x) => [x.key, x.en]));
const chiaviUi = new Set(ui.map((x) => x.key));

const segni = (s) =>
  [...s.matchAll(/\{\w+\}|\[\[[\w.]+\]\]/g)]
    .map((m) => m[0])
    .sort()
    .join('|');
const righe = leggiCsv(readFileSync(file, 'utf8'));
const testa = righe.shift() ?? [];
if (testa[0]?.trim() !== 'key' || testa.length < 3) throw new Error("Il file non ha l'intestazione key,English,Dutch");

const nlUi = {};
const nlTesti = {};
let tradotte = 0;
let scartate = 0;
let sconosciute = 0;
for (const [key, , olandese = ''] of righe) {
  const k = key.trim();
  if (!inglese.has(k)) {
    sconosciute++;
    console.warn(`chiave sconosciuta, scartata: ${k}`);
    continue;
  }
  if (olandese.trim() === '') continue;
  if (segni(inglese.get(k)) !== segni(olandese)) {
    scartate++;
    console.warn(`segnaposto diversi dall'inglese, voce saltata: ${k}\n  EN: ${inglese.get(k)}\n  NL: ${olandese}`);
    continue;
  }
  (chiaviUi.has(k) ? nlUi : nlTesti)[k] = olandese;
  tradotte++;
}
const nota = 'Generato da `scripts/importa-nl.mjs` dal foglio `docs/traduzione-olandese.csv`: non si modifica a mano.';
writeFileSync(FILE_UI, scriviDizionario('NL_UI', `Olandese: testi dell'interfaccia, per le stesse chiavi di \`STRINGS\`. ${nota}`, nlUi), 'utf8');
writeFileSync(FILE_TESTI, scriviDizionario('NL_TEXTS', `Olandese: Guida e Novita' (testi senza nome, chiave \`t.<impronta del testo inglese>\`). ${nota}`, nlTesti), 'utf8');
console.log(`importate ${tradotte} voci (interfaccia ${Object.keys(nlUi).length}, Guida e Novita' ${Object.keys(nlTesti).length}); saltate ${scartate} per segnaposto; scartate ${sconosciute} chiavi sconosciute`);
