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

  it('segni di procedura: un segno solo in CW, lettere negli altri modi', () => {
    expect(runMacro('TNX 73 <SK>', { ...ctx, modeName: 'CW' }).text).toBe('TNX 73 <SK>');
    expect(runMacro('<AR> <BT> <KN>', { ...ctx, modeName: 'CW' }).text).toBe('<AR> <BT> <KN>');
    expect(runMacro('TNX 73 <SK>', ctx).text).toBe('TNX 73 SK');
    expect(runMacro('TNX 73 <SK>', ctx).actions).toEqual([]);
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
    const r = runMacro('<TX>CQ<LETTERS><EXEC:Backup.bat>', ctx);
    expect(r.text).toBe('CQ');
    expect(r.actions).toEqual([{ type: 'tx' }, { type: 'unknown', tag: 'LETTERS' }, { type: 'unknown', tag: 'EXEC:Backup.bat' }]);
  });

  it('CATCMD manda la stringa alla radio; IMD e SP non scrivono niente', () => {
    const r = runMacro('<CATCMD:AN1;><SP>ok<IMD>', ctx);
    expect(r.text).toBe('ok');
    expect(r.actions).toEqual([{ type: 'cat', cmd: 'AN1;' }]);
  });

  it('ultimo QSO e file di testo vengono dal contesto', () => {
    const withData = { ...ctx, lastQso: { date: '20260101', band: '20m', mode: 'RTTY' }, files: { 'wx.txt': 'sole' } };
    expect(runMacro('<ANYQSOBEFOREDATE> <ANYQSOBEFOREBAND> <ANYQSOBEFOREMODE> <TEXTFILE:wx.txt>', withData).text).toBe('20260101 20m RTTY sole');
    expect(runMacro('[<ANYQSOBEFOREDATE>][<TEXTFILE:wx.txt>]', ctx).text).toBe('[][]');
  });

  it('la velocita della CW, assoluta o relativa', () => {
    expect(runMacro('<WPM:20><WPM:+3><WPM:-2>', ctx).actions).toEqual([
      { type: 'wpm', wpm: 20, relative: false },
      { type: 'wpm', wpm: 3, relative: true },
      { type: 'wpm', wpm: -2, relative: true },
    ]);
  });

  it('AFC, Snap, Squelch, Lock, Scan, memorie e finestre', () => {
    expect(runMacro('<AFCON><AFCOFF><AFC>', ctx).actions).toEqual([
      { type: 'toggle', what: 'afc', value: true },
      { type: 'toggle', what: 'afc', value: false },
      { type: 'toggle', what: 'afc', value: 'toggle' },
    ]);
    expect(runMacro('<SNAP><SQUELCHON><LOCKTX><UNLOCKTX>', ctx).actions).toEqual([
      { type: 'toggle', what: 'snap', value: 'toggle' },
      { type: 'toggle', what: 'squelch', value: true },
      { type: 'toggle', what: 'lock', value: true },
      { type: 'toggle', what: 'lock', value: false },
    ]);
    expect(runMacro('<STARTSCAN><STOPSCAN><MEMW:3><MEMR:20><MODESETTINGS><THRESHOLD>', ctx).actions).toEqual([
      { type: 'scan', run: true },
      { type: 'scan', run: false },
      { type: 'memory', write: true, slot: 3 },
      { type: 'memory', write: false, slot: 20 },
      { type: 'modeSettings' },
      { type: 'threshold' },
    ]);
  });

  it('una memoria fuori da 1-20 non e\' valida', () => {
    expect(runMacro('<MEMW:0>', ctx).actions).toEqual([{ type: 'unknown', tag: 'MEMW:0' }]);
    expect(runMacro('<MEMR:21>', ctx).actions).toEqual([{ type: 'unknown', tag: 'MEMR:21' }]);
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
