// Scrive docs/traduzione-olandese.csv: chiave, inglese, olandese (UTF-8 con BOM) per tutti i testi dell'interfaccia, della Guida e delle Novita'.
// L'olandese gia' presente nel dizionario viene riportato; le voci non ancora tradotte hanno la colonna vuota.
// Uso: node scripts/export-nl.mjs
import { writeFileSync } from 'node:fs';
import { FILE_CSV, caricaModello, csvCampo } from './nl-modello.mjs';

const { ui, testi, nlUi, nlTesti } = await caricaModello();
const righe = [['key', 'English', 'Dutch']];
for (const { key, en } of ui) righe.push([key, en, nlUi[key] ?? '']);
for (const { key, en } of testi) righe.push([key, en, nlTesti[key] ?? '']);
writeFileSync(FILE_CSV, `﻿${righe.map((r) => r.map(csvCampo).join(',')).join('\r\n')}\r\n`, 'utf8');
const fatte = righe.slice(1).filter((r) => r[2] !== '').length;
console.log(`${FILE_CSV}: ${righe.length - 1} voci (${ui.length} interfaccia, ${testi.length} Guida e Novita'), ${fatte} tradotte`);
