import { describe, expect, it } from 'vitest';
import { cssColor, effectiveMacros, keyNames, macroNameForKey, macroNeeds, mainCall, runMacro, textColorFor, type MacroContext } from './macroLang';

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
    expect(r.actions).toEqual([{ type: 'tx' }, { type: 'exec', file: 'Backup.bat' }]);
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

  it('<GRABCALL> diventa un comando con il nominativo trovato', () => {
    expect(runMacro('<GRABCALL:K1ABC>', ctx).actions).toEqual([{ type: 'grabCall', call: 'K1ABC' }]);
    expect(runMacro('<GRABCALL:>', ctx).actions).toEqual([{ type: 'grabCall', call: '' }]);
  });

  it('numero mandato e QSO già fatto vengono dal log del contest', () => {
    const withContest = { ...ctx, contestPrev: { lastSent: '057', beforeMs: Date.UTC(2026, 10, 15, 14, 32) } };
    expect(runMacro('NR <PREVNRS> QSO B4 <QSOBEFOREDATE> <QSOBEFORETIME>', withContest).text).toBe('NR 057 QSO B4 2026-11-15 14:32');
    expect(runMacro('[<PREVNRS>][<QSOBEFORETIME>]', ctx).text).toBe('[][]');
    const last = { ...ctx, contestPrev: { lastSent: '057', beforeMs: null, lastCall: 'K1ABC', lastRst: '599' } };
    expect(runMacro('<PREVCALL> <PREVRSTS> <PREVNRS>', last).text).toBe('K1ABC 599 057');
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
    expect(runMacro('Hello world', ctx)).toEqual({ text: 'Hello world', actions: [], steps: [{ text: 'Hello world' }] });
  });
});

describe('tasti e insiemi', () => {
  it('48 tasti nell\'ordine F1-F12, Ctrl, Shift, Ctrl-Shift', () => {
    const names = keyNames();
    expect(names).toHaveLength(48);
    expect(names[0]).toBe('F1');
    expect(names[12]).toBe('Ctrl-F1');
    expect(names[24]).toBe('Shift-F1');
    expect(names[47]).toBe('Ctrl-Shift-F12');
  });

  it('il nome della macro da un tasto premuto', () => {
    expect(macroNameForKey({ key: 'F5', ctrlKey: false, shiftKey: false })).toBe('F5');
    expect(macroNameForKey({ key: 'F12', ctrlKey: true, shiftKey: true })).toBeNull();
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

describe('IGNOREBEFORE e PERIOD', () => {
  it('diventano azioni', () => {
    expect(runMacro('<IGNOREBEFORE:2026-11-14 12:30>', ctx).actions).toEqual([{ type: 'ignoreBefore', at: Date.UTC(2026, 10, 14, 12, 30) }]);
    expect(runMacro('<IGNOREBEFORE:ERASE>', ctx).actions).toEqual([{ type: 'ignoreBefore', at: 'erase' }]);
    expect(runMacro('<PERIOD:60>', ctx).actions).toEqual([{ type: 'period', minutes: 60 }]);
  });
});

describe('EXEC', () => {
  it('riconosce <EXEC:...> come comando non supportato per sicurezza, senza eseguire niente', () => {
    const r = runMacro('<EXEC:Backup.bat>CQ', ctx);
    expect(r.actions).toEqual([{ type: 'exec', file: 'Backup.bat' }]);
    expect(r.text).toBe('CQ');
  });
});

const cmdOf = (script: string, c: Partial<MacroContext> = {}) => runMacro(script, { ...ctx, ...c }).actions;

describe('macro dentro una macro e ?', () => {
  const macros = new Map([['F2', 'CQ <MYCALL>'], ['F3', '<F2> K'], ['LOOP', '<LOOP>'], ['GE', 'Buonasera']]);
  it('espande le macro nominate, anche annidate', () => {
    expect(runMacro('<F3> <TX>', { ...ctx, macros }).text).toBe('CQ IU3ABC K');
  });
  it('si ferma oltre 10 livelli con un avviso', () => {
    const r = runMacro('<LOOP>', { ...ctx, macros });
    expect(r.actions).toEqual([{ type: 'alarm', text: 'MACRO: max 10' }]);
  });
  it('il ? apre la macro; un tasto senza macro non fa niente; il minuscolo sconosciuto resta testo', () => {
    expect(cmdOf('<F5?>')).toEqual([{ type: 'editMacro', name: 'F5' }]);
    expect(runMacro('<F7>', ctx).actions).toEqual([]);
    expect(runMacro('a<br>b', ctx).text).toBe('a<br>b');
  });
  it('macroNeeds trova file, campi da chiedere e GRABCALL anche nelle macro annidate', () => {
    const n = macroNeeds('<F9> <CALL?>', new Map([['F9', '<TEXTFILE:a.txt><GRABCALL><INIFILE:b.ini,S,K>']]));
    expect(n.files.sort()).toEqual(['a.txt', 'b.ini']);
    expect(n.ask).toEqual(['CALL']);
    expect(n.grab).toBe(true);
  });
});

describe('corrispondente e QSO precedente', () => {
  const prevQso = { call: 'F5ABC/P', name: 'Jean', qth: 'Paris', rsts: '589', rstr: '579', nrr: '015', ms: Date.UTC(2026, 10, 9, 8, 7) };
  it('ACALL, CALLPOST, POST, OM, DIST, AZ', () => {
    expect(runMacro('<ACALL><CALLPOST:?> <POST:YES,NO> <OM>', ctx).text).toBe('DL1XYZDL1XYZ? YES OM');
    expect(runMacro('<POST:YES,NO>', { ...ctx, call: '' }).text).toBe('NO');
    expect(runMacro('<OM>', { ...ctx, name: 'Maria' }).text).toBe('YL');
    expect(runMacro('<DIST> <AZ>', { ...ctx, wwloc: '' }).text).toBe('? ?');
    expect(Number(runMacro('<DIST>', ctx).text)).toBeGreaterThan(500);
  });
  it('dati dell\'ultimo QSO e LASTCALL', () => {
    expect(runMacro('<PREVNAME> <PREVQTH> <PREVRSTR> <PREVCTIME> <PREVSUF2> <PREVCCALL> <PREVNRR> <PREVCALL>', { ...ctx, prevQso }).text).toBe('Jean Paris 579 0807 BC F5ABC 015 F5ABC/P');
    expect(cmdOf('<LASTCALL>', { prevQso })).toEqual([{ type: 'setField', field: 'call', value: 'F5ABC/P' }]);
  });
  it('NRRPOST, QSOBEFORE e ONQSOBEFORE', () => {
    expect(runMacro('<NRRPOST:!>', { ...ctx, nrr: '5NN TT1' }).text).toBe('599 001!');
    const before = { ms: Date.UTC(2026, 10, 1, 10, 0), band: '20m', mode: 'PSK31' };
    expect(runMacro('<QSOBEFORE> <QSOBEFORETIME1> <QSOBEFOREBAND> <QSOBEFOREMODE>', { ...ctx, before }).text).toBe('2026-11-01 10:00 1000 20m PSK31');
    const macros = new Map([['F9', 'QSO B4']]);
    expect(runMacro('<ONQSOBEFORE:F9>', { ...ctx, before, macros }).text).toBe('QSO B4');
    expect(runMacro('<ONQSOBEFORE:F9>', { ...ctx, macros }).text).toBe('');
  });
});

describe('testo selezionato', () => {
  it('SEL... mettono la selezione nei campi e valgono subito', () => {
    const r = runMacro('<SELCALL><CALL> <SELRXTEXT>', { ...ctx, selection: ' k1abc ' });
    expect(r.text).toBe('K1ABC k1abc');
    expect(r.actions).toEqual([{ type: 'setField', field: 'call', value: 'K1ABC' }]);
    expect(cmdOf('<SELNAME>', { selection: 'Jean Paul' })).toEqual([{ type: 'setField', field: 'name', value: 'Jean Paul' }]);
    expect(cmdOf('<SAVESELTEXT>')).toEqual([{ type: 'cmd', tag: 'SAVESELTEXT', arg: '' }]);
  });
});

describe('frasi automatiche', () => {
  it('nella lingua del prefisso o in quella imposta', () => {
    expect(runMacro('<INTQSL> <INTDE>', { ...ctx, call: 'F5ABC' }).text).toBe('QSL, merci de');
    expect(runMacro('<INTQSL>', ctx).text).toBe('QSL, danke');
    expect(runMacro('<LANG_IT><INTQSL>', ctx).text).toBe('QSL, grazie');
    expect(cmdOf('<FORCELANGUAGE:ES>')).toEqual([{ type: 'cmd', tag: 'FORCELANGUAGE', arg: 'es' }]);
    expect(runMacro('<INTE>', { ...ctx, lang: 'nl' }).text).toBe('en');
  });
  it('GM/GE/GN usano la macro utente del momento', () => {
    const evening = new Date(2026, 10, 10, 19, 0);
    expect(runMacro('<GM>', { ...ctx, now: evening, macros: new Map([['GE', 'Buonasera']]) }).text).toBe('Buonasera');
    expect(runMacro('<GN>', { ...ctx, now: evening }).text).toBe('GE');
  });
});

describe('ora e data', () => {
  it('formati, TXTIME, STARTTIME e TIMER', () => {
    expect(runMacro('<TIME:%H%M%S> <DATE:%d/%m/%y> <DATE:%j>', ctx).text).toBe('140507 10/11/26 314');
    expect(runMacro('<TXTIME>', { ...ctx, txStart: new Date(Date.UTC(2026, 10, 10, 13, 2)) }).text).toBe('13:02');
    expect(cmdOf('<STARTTIME:1230><TIMER:60,F2>')).toEqual([{ type: 'cmd', tag: 'STARTTIME', arg: '1230' }, { type: 'cmd', tag: 'TIMER', arg: '60,F2' }]);
    expect(runMacro('<STARTTIME:%H%M>', ctx).text).toBe('0930');
  });
});

describe('QSO e log', () => {
  it('scambio, salvataggio e comandi del log', () => {
    expect(cmdOf('<INCNRR>', { nrr: '009' })).toEqual([{ type: 'setField', field: 'nrr', value: '010' }]);
    expect(cmdOf('<CLEARNRR>')).toEqual([{ type: 'setField', field: 'nrr', value: '' }]);
    expect(cmdOf('<SAVE><SAVESHAUTO>')).toEqual([{ type: 'saveQso' }, { type: 'saveQso' }, { type: 'clearQso' }]);
    expect(cmdOf('<ERASE>')).toEqual([{ type: 'ignoreBefore', at: 'erase' }]);
    expect(cmdOf('<WKDIGNOREBEFORE:ERASE>')).toEqual([{ type: 'ignoreBefore', at: 'erase' }]);
    expect(cmdOf('<SENDSPOT:TNX>')).toEqual([{ type: 'cmd', tag: 'SENDSPOT', arg: 'TNX' }]);
    expect(runMacro('<CORRECTCALLTX>', ctx).text).toBe('DL1XYZ');
  });
});

describe('trasmissione, modo e demodulatore', () => {
  it('SET, GET, TEST, CWID e MODEADD', () => {
    expect(cmdOf('<SET BW:250><SET SUBMODE:MFSK32>')).toEqual([{ type: 'cmd', tag: 'SET', arg: 'BW:250' }, { type: 'cmd', tag: 'SET', arg: 'SUBMODE:MFSK32' }]);
    expect(runMacro('S/N <GET SN> OFS <GET OFFSET> RATE <GET RATE>', { ...ctx, snr: 12.4, rxHz: 1500, rate: '31.25' }).text).toBe('S/N 12 OFS 1500 RATE 31.25');
    expect(runMacro('CQ <TEST>', { ...ctx, inContest: true }).text).toBe('CQ TEST');
    expect(runMacro('73<CWID>', ctx).text).toBe('73 DE IU3ABC');
    expect(cmdOf('<MODEADD:rtty><LOCKMODE><INVON><PTT:OFF>')).toEqual([{ type: 'mode', mode: 'RTTY' }, { type: 'cmd', tag: 'LOCKMODE', arg: '' }, { type: 'cmd', tag: 'INVON', arg: '' }, { type: 'cmd', tag: 'PTT', arg: 'OFF' }]);
  });
});

describe('CAT', () => {
  it('esadecimale, comandi grezzi e attese', () => {
    expect(cmdOf('<CATCMDHEX:FE FE 94 E0 03 FD>')).toEqual([{ type: 'cat', cmd: '\\0xFE\\0xFE\\0x94\\0xE0\\0x03\\0xFD' }]);
    expect(cmdOf('<CATCMDX:FA;><CATWAIT:200>')).toEqual([{ type: 'cat', cmd: 'FA;' }, { type: 'cmd', tag: 'CATWAIT', arg: '200' }]);
    expect(cmdOf('<CATCMDHEX:ZZ>')).toEqual([{ type: 'cmd', tag: 'CATBAD', arg: 'ZZ' }]);
  });
});

describe('WinKeyer', () => {
  it('testo, byte e potenziometro', () => {
    expect(cmdOf('<WKTEXT:CQ><WK:TEST><WKHEX:02 1E><WKNOPOT>')).toEqual([
      { type: 'cmd', tag: 'WKTEXT', arg: 'CQ' },
      { type: 'cmd', tag: 'WKTEXT', arg: 'TEST' },
      { type: 'cmd', tag: 'WKHEX', arg: '02 1E' },
      { type: 'cmd', tag: 'WKNOPOT', arg: '' },
    ]);
  });
});

describe('rotore', () => {
  it('comandi GS-232 in testo ed esadecimale', () => {
    expect(cmdOf('<ROTORCMD:M180><ROTORCMD:S><ROTORCMDHEX:4D 30 39 30>')).toEqual([{ type: 'rotor', az: 180 }, { type: 'cmd', tag: 'ROTORSTOP', arg: '' }, { type: 'rotor', az: 90 }]);
    expect(cmdOf('<ROTORCMD:XYZ>')).toEqual([{ type: 'cmd', tag: 'ROTORBAD', arg: 'XYZ' }]);
  });
});





