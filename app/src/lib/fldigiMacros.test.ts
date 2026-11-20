import { describe, expect, it } from 'vitest';
import type { DigiMacro } from '../api/client';
import { applyImport, buildImport, convertFldigiText, parseFldigiFile } from './fldigiMacros';

/** Un file nel formato di fldigi (intestazione `extended`, commenti, macro con `\n` a fine riga e testo su piu' righe). */
const FILE = [
  '//fldigi macro definition file extended',
  '// This file defines the macro structure(s) for the digital modem program, fldigi',
  '//',
  '//',
  '// Macro # 1',
  '/$ 0 CQ',
  '<TX>CQ CQ CQ DE <MYCALL> <MYCALL> <MYCALL> PSE K\\n',
  '<RX>',
  '//',
  '// Macro # 2',
  '/$ 1 ANS',
  '<CALL> DE <MYCALL> TNX CALL <NAME> UR RST <RST> <WPM:25> <MYRST> KN<RX>',
  '//',
  '// Macro # 13',
  '/$ 12 73',
  '73 <NAME> DE <MYCALL> <LOG><CLRQSO>',
  '//',
  '// Macro # 3',
  '/$ 2 ',
  '',
].join('\n');

const settings = (macros: DigiMacro[], mode_macros: Record<string, DigiMacro[]> = {}) => ({ macros, mode_macros, mode: 'psk31' as const });

describe('parseFldigiFile', () => {
  it('riconosce l intestazione e legge numero, etichetta e testo (le righe si attaccano, \\n e un a-capo)', () => {
    const f = parseFldigiFile(FILE);
    expect(f).not.toBeNull();
    expect(f?.extended).toBe(true);
    const cq = f?.macros[0];
    expect(cq?.label).toBe('CQ');
    expect(cq?.text).toBe('<TX>CQ CQ CQ DE <MYCALL> <MYCALL> <MYCALL> PSE K\n<RX>');
    expect(f?.macros[12]?.label).toBe('73');
  });

  it('un file che non e di fldigi si rifiuta', () => {
    expect(parseFldigiFile('Aurora macros\n/$ 0 CQ\nCQ')).toBeNull();
    expect(parseFldigiFile('')).toBeNull();
  });

  it('nei file vecchi (senza «extended») le macro dalla 10 in poi sono spostate di due posti', () => {
    const old = ['//fldigi macro definition file', '/$ 9 NINE', 'nine', '/$ 10 TEN', 'ten', ''].join('\n');
    const f = parseFldigiFile(old);
    expect(f?.extended).toBe(false);
    expect(f?.macros[9]?.label).toBe('NINE');
    expect(f?.macros[12]?.label).toBe('TEN');
  });

  it('un numero fuori da 0-47 ferma la lettura senza errori', () => {
    const f = parseFldigiFile(['//fldigi macro definition file extended', '/$ 0 A', 'a', '/$ 99 BAD', 'bad', ''].join('\n'));
    expect(f?.macros[0]?.label).toBe('A');
    expect(f?.macros.filter((m) => m.label !== '').length).toBe(1);
  });

  it('accetta i fine riga di Windows e il segno di ordine dei byte', () => {
    const f = parseFldigiFile(`﻿${FILE.replace(/\n/g, '\r\n')}`);
    expect(f?.macros[1]?.label).toBe('ANS');
  });
});

describe('convertFldigiText', () => {
  it('traduce i tag con lo stesso significato e lascia gli altri, elencandoli', () => {
    const r = convertFldigiText('<TX><CALL> DE <MYCALL> <MYLOC> <LOC> <RST> <MYRST> <CNTR> <CLRQSO><LOG><RX> <WPM:25> <ID>');
    expect(r.text).toBe('<TX><CALL> DE <MYCALL> <MYWWLOC> <WWLOC> <RSTS> <RSTR> <QSONR> <CLEARQSO><SAVEQSO><RX> <WPM:25> <ID>');
    expect(r.untranslated).toEqual(['<WPM:25>', '<ID>']);
  });

  it('non tocca il testo senza tag e non confonde i segni di minore e maggiore', () => {
    expect(convertFldigiText('A < B > C').text).toBe('A < B > C');
    expect(convertFldigiText('ciao').untranslated).toEqual([]);
  });

  it('i tag si riconoscono anche in minuscolo', () => {
    expect(convertFldigiText('<mycall>').text).toBe('<MYCALL>');
  });
});

describe('buildImport', () => {
  it('mette le macro sui tasti di Aurora per numero (12 = Ctrl-F1) e salta le vuote', () => {
    const f = parseFldigiFile(FILE);
    expect(f).not.toBeNull();
    const r = buildImport(f!);
    expect(r.macros.map((m) => m.name)).toEqual(['F1', 'F2', 'Ctrl-F1']);
    expect(r.macros[0]?.text).toContain('<MYCALL>');
    expect(r.untranslated).toEqual(['<WPM:25>']);
    // 48 posti: 3 macro importate e 45 vuote (la numero 2 del file non ha ne etichetta ne testo)
    expect(r.skipped).toBe(45);
  });
});

describe('applyImport', () => {
  const imported = [
    { name: 'F1', label: 'CQ', text: 'CQ <MYCALL>' },
    { name: 'F2', label: 'ANS', text: '<CALL> DE <MYCALL>' },
  ];
  const base: DigiMacro[] = [
    { name: 'F1', label: 'vecchia', text: 'x', color: 3 },
    { name: 'F3', label: 'resta', text: 'y', color: null },
  ];

  it('aggiungi: solo i tasti del file cambiano, gli altri restano; il gruppo del modo non si tocca', () => {
    const modeSet = { psk31: [{ name: 'F9', label: 'm', text: 'm', color: null }] };
    const r = applyImport(settings(base, modeSet), imported, 'all', 'add');
    expect(r.macros.map((m) => m.name).sort()).toEqual(['F1', 'F2', 'F3']);
    expect(r.macros.find((m) => m.name === 'F1')?.label).toBe('CQ');
    expect(r.macros.find((m) => m.name === 'F3')?.label).toBe('resta');
    expect(r.mode_macros).toEqual(modeSet);
  });

  it('sostituisci: il gruppo diventa solo quello del file', () => {
    const r = applyImport(settings(base), imported, 'all', 'replace');
    expect(r.macros.map((m) => m.name)).toEqual(['F1', 'F2']);
  });

  it('solo per il modo in uso: tocca il gruppo del modo e lascia le macro di base', () => {
    const r = applyImport(settings(base, { psk31: [{ name: 'F2', label: 'x', text: 'x', color: null }, { name: 'F5', label: 'k', text: 'k', color: null }] }), imported, 'mode', 'add');
    expect(r.macros).toEqual(base);
    expect(r.mode_macros.psk31?.map((m) => m.name).sort()).toEqual(['F1', 'F2', 'F5']);
    expect(r.mode_macros.psk31?.find((m) => m.name === 'F2')?.label).toBe('ANS');
  });
});
