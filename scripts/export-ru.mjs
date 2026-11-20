// Scrive docs/traduzione-russa.csv: chiave, inglese, russo (UTF-8 con BOM) per tutti i testi dell'interfaccia, della Guida e delle ultime 10 Novita' (le altre restano in inglese).
// Il russo gia' presente nel dizionario viene riportato; le voci non ancora tradotte hanno la colonna vuota.
// Uso: node scripts/export-ru.mjs
import { writeFileSync } from 'node:fs';
import { FILE_CSV, caricaModello, csvCampo } from './ru-modello.mjs';

const { ui, testi, ruUi, ruTesti } = await caricaModello();
const righe = [['key', 'English', 'Russian']];
for (const { key, en } of ui) righe.push([key, en, ruUi[key] ?? '']);
for (const { key, en } of testi) righe.push([key, en, ruTesti[key] ?? '']);
writeFileSync(FILE_CSV, `﻿${righe.map((r) => r.map(csvCampo).join(',')).join('\r\n')}\r\n`, 'utf8');
const fatte = righe.slice(1).filter((r) => r[2] !== '').length;
console.log(`${FILE_CSV}: ${righe.length - 1} voci (${ui.length} interfaccia, ${testi.length} Guida e Novita' recenti), ${fatte} tradotte`);
