import { describe, expect, it } from 'vitest';
import { cssColor, effectiveMacros, keyNames, macroNameForKey, mainCall, runMacro, textColorFor, type MacroContext } from './macroLang';

const ctx: MacroContext = {
  mycall: 'iu3abc',
  myname: 'Gianni',
  myqth: 'Verona',
  mywwloc: 'jn55',
  call: 'dl1xyz',
  name: 'Hans',
  qth: 'Berlin',
  wwloc: 'jo62',
  rstr: '579',
  rsts: '599',
  notes: '',
  nrr: '12',
  nrs: '34',
  startTime: new Date(Date.UTC(2026, 10, 10, 9, 30)),
  now: new Date(Date.UTC(2026, 10, 10, 14, 5, 7)),
  dialHz: 14_070_000,
  band: '20m',
  modeName: 'PSK31',
  qsoNumber: 7,
};

describe('runMacro: comandi di testo', () => {
  it('riempie i dati della stazione e del corrispondente', () => {
    const r = runMacro('<CALL> DE <MYCALL> UR <RSTS> NAME <MYNAME> QTH <MYQTH> LOC <MYWWLOC>', ctx);
    expect(r.text).toBe('DL1XYZ DE IU3ABC UR 599 NAME Gianni QTH Verona LOC JN55');
    expect(r.actions).toEqual([]);
  });

  it('data, ora, frequenza, banda e modo', () => {
    expect(runMacro('<DATE> <TIME> <CTIME>', ctx).text).toBe('2026-11-10 14:05 1405');
    expect(runMacro('<KHZ> <KHZ:2> <MHZ:3> <BAND> <MODE>', ctx).text).toBe('14070 14070.00 14.070 20m PSK31');
    expect(runMacro('<STARTTIME>', ctx).text).toBe('09:30');
  });

  it('saluto secondo l\'ora', () => {
    expect(runMacro('<GA>', ctx).text).toBe('GA');
    expect(runMacro('<GAL>', { ...ctx, now: new Date(Date.UTC(2026, 0, 1, 7)) }).text).toBe('Good Morning');
  });

  it('a capo, tabulazione e parentesi', () => {
    expect(runMacro('A<CR>B<LF>C<CRLF>D<TAB>E<[>F<]>', ctx).text).toBe('A\rB\nC\r\nD\tE<F>');
  });

  it('il nominativo senza suffissi', () => {
    expect(runMacro('<MYMAINCALL>', { ...ctx, mycall: 'iu3abc/p' }).text).toBe('IU3ABC');
    expect(mainCall('DL/IU3ABC/M')).toBe('IU3ABC');
  });

  it('un dato vuoto non lascia spazi doppi', () => {
    expect(runMacro('<CALL> DE <MYCALL> <NOTES> K', ctx).text).toBe('DL1XYZ DE IU3ABC K');
  });
});

describe('runMacro: comandi di programma', () => {
  it('«Call»: TX, il testo e RX', () => {
    const r = runMacro('<TX><CALL> de <MYCALL> k<RX>', ctx);
    expect(r.text).toBe('DL1XYZ de IU3ABC k');
    expect(r.actions).toEqual([{ type: 'tx' }, { type: 'rx', clear: false }]);
  });

  it('«Bye»: salva il QSO e torna in RX pulendo', () => {
    const r = runMacro('<TX>73 de <MYCALL> sk<SAVEQSO><RXANDCLEAR>', ctx);
    expect(r.actions).toEqual([{ type: 'tx' }, { type: 'saveQso' }, { type: 'rx', clear: true }]);
  });

  it('cambio di modo, velocita\' e spostamento', () => {
    expect(runMacro('<MODE:RTTY><CR><LF><CLEARTXWINDOW>', ctx).actions).toEqual([{ type: 'mode', mode: 'RTTY' }, { type: 'clearTx' }]);
    expect(runMacro('<BAUDRATE:62.5>', ctx).actions).toEqual([{ type: 'baud', baud: 62.5 }]);
    expect(runMacro('<SHIFT:170><AUDIOFQ:-50><AUDIOFQ:1500>', ctx).actions).toEqual([
      { type: 'shift', hz: 170 },
      { type: 'audioFq', hz: -50, relative: true },
      { type: 'audioFq', hz: 1500, relative: false },
    ]);
  });

  it('i comandi non ancora fatti restano nell\'elenco', () => {
    const r = runMacro('<TX>CQ<WPM:20><CATCMD:AN1;>', ctx);
    expect(r.text).toBe('CQ');
    expect(r.actions).toEqual([{ type: 'tx' }, { type: 'unknown', tag: 'WPM:20' }, { type: 'unknown', tag: 'CATCMD:AN1;' }]);
  });

  it('il testo senza comandi passa com\'e\'', () => {
    expect(runMacro('Hello world', ctx)).toEqual({ text: 'Hello world', actions: [] });
  });
});

describe('tasti e insiemi', () => {
  it('48 tasti nell\'ordine', () => {
    const names = keyNames();
    expect(names).toHaveLength(48);
    expect(names[0]).toBe('F1');
    expect(names[12]).toBe('Ctrl-F1');
    expect(names[24]).toBe('Shift-F1');
    expect(names[47]).toBe('Ctrl-Shift-F12');
  });

  it('il nome della macro da un tasto premuto', () => {
    expect(macroNameForKey({ key: 'F5', ctrlKey: false, shiftKey: false })).toBe('F5');
    expect(macroNameForKey({ key: 'F12', ctrlKey: true, shiftKey: true })).toBe('Ctrl-Shift-F12');
    expect(macroNameForKey({ key: 'a', ctrlKey: false, shiftKey: false })).toBeNull();
    expect(macroNameForKey({ key: 'F13', ctrlKey: false, shiftKey: false })).toBeNull();
  });

  it('le macro del modo sostituiscono quelle di base con lo stesso nome', () => {
    const base = [{ name: 'F10', label: 'RX', text: '<RX>', color: null }, { name: 'F1', label: 'CQ', text: 'CQ', color: null }];
    const mode = [{ name: 'F10', label: 'PSK31', text: '<BAUDRATE:31.25>', color: null }];
    const m = effectiveMacros(base, mode);
    expect(m.get('F10')?.label).toBe('PSK31');
    expect(m.get('F1')?.label).toBe('CQ');
    expect(effectiveMacros(base, undefined).get('F10')?.label).toBe('RX');
  });

  it('colori: CSS e testo leggibile', () => {
    expect(cssColor(0xff8000)).toBe('#ff8000');
    expect(cssColor(null)).toBeUndefined();
    expect(textColorFor(0xffff80)).toBe('#101418');
    expect(textColorFor(0x000080)).toBe('#ffffff');
  });
});
