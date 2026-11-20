// Rilegge docs/traduzione-russa.csv (o il file dato) e riscrive il dizionario russo (app/src/i18n/ru/ui.ts e texts.ts).
// Le voci con la colonna «Russian» vuota non entrano nel dizionario (in app ricadono sull'inglese). Le chiavi che non esistono piu' si scartano.
// Controlla che i segnaposto {nome} e i riferimenti [[chiave]] dell'inglese ci siano anche nel russo: se no, avvisa e salta la voce.
// Uso: node scripts/importa-ru.mjs [file.csv]
import { readFileSync, writeFileSync } from 'node:fs';
import { FILE_CSV, FILE_TESTI, FILE_UI, caricaModello, leggiCsv, scriviDizionario } from './ru-modello.mjs';

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
if (testa[0]?.trim() !== 'key' || testa.length < 3) throw new Error("Il file non ha l'intestazione key,English,Russian");

const ruUi = {};
const ruTesti = {};
let tradotte = 0;
let scartate = 0;
let sconosciute = 0;
for (const [key, , russo = ''] of righe) {
  const k = key.trim();
  if (!inglese.has(k)) {
    sconosciute++;
    console.warn(`chiave sconosciuta, scartata: ${k}`);
    continue;
  }
  if (russo.trim() === '') continue;
  if (segni(inglese.get(k)) !== segni(russo)) {
    scartate++;
    console.warn(`segnaposto diversi dall'inglese, voce saltata: ${k}\n  EN: ${inglese.get(k)}\n  RU: ${russo}`);
    continue;
  }
  (chiaviUi.has(k) ? ruUi : ruTesti)[k] = russo;
  tradotte++;
}
const nota = 'Generato da `scripts/importa-ru.mjs` dal foglio `docs/traduzione-russa.csv`: non si modifica a mano.';
writeFileSync(FILE_UI, scriviDizionario('RU_UI', `Russo: testi dell'interfaccia, per le stesse chiavi di \`STRINGS\`. ${nota}`, ruUi), 'utf8');
writeFileSync(FILE_TESTI, scriviDizionario('RU_TEXTS', `Russo: Guida e Novita' (testi senza nome, chiave \`t.<impronta del testo inglese>\`). ${nota}`, ruTesti), 'utf8');
console.log(`importate ${tradotte} voci (interfaccia ${Object.keys(ruUi).length}, Guida e Novita' ${Object.keys(ruTesti).length}); saltate ${scartate} per segnaposto; scartate ${sconosciute} chiavi sconosciute`);
