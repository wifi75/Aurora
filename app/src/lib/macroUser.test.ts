import { describe, expect, it } from 'vitest';
import { runMacro, type MacroContext } from './macroLang';

const ctx: MacroContext = { mycall: 'iu3abc', myname: 'Tiz', myqth: 'X', mywwloc: 'io91wm', call: 'ec7yy', name: '', qth: '', wwloc: '', rstr: '599', rsts: '599', notes: '', nrr: '015', nrs: '007', startTime: null, now: new Date('2026-11-14T12:00:00Z'), dialHz: 21_091_000, band: '15m', modeName: 'RTTY', qsoNumber: 7 };

describe('macro di contest', () => {
  it('<TX><FIXSTARTTIME> de <MYCALL> UR 5NN  TU<RXANDCLEAR>', () => {
    const r = runMacro('<TX><FIXSTARTTIME> de <MYCALL> UR 5NN  TU<RXANDCLEAR>', ctx);
    expect(r.text).toBe('de IU3ABC UR 5NN TU');
    expect(r.actions).toEqual([{ type: 'tx' }, { type: 'fixStartTime' }, { type: 'rx', clear: true }]);
  });
  it('<CALL> de <MYCALL> 599 <QSONR> <QSONR> e scambi', () => {
    const r = runMacro('<TX><CALL> 599 <QSONR> <QSONR> <NRS> <NRR> TU<RX>', ctx);
    expect(r.text).toMatch(/^EC7YY 599 007 007 007 015 TU$/);
    expect(r.actions.map((a) => a.type)).toEqual(['tx', 'rx']);
  });
});
