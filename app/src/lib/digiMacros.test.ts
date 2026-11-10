import { describe, expect, it } from 'vitest';
import { expandMacro } from './digiMacros';

const fields = { mycall: 'iu3abc', call: 'dl1xyz', rst: '599', name: 'Gianni', qth: 'Verona' };

describe('expandMacro', () => {
  it('riempie i dati e mette tutto in maiuscolo', () => {
    expect(expandMacro('CQ CQ DE <MYCALL> <MYCALL> K', fields)).toBe('CQ CQ DE IU3ABC IU3ABC K');
    expect(expandMacro('<CALL> DE <MYCALL> UR RST <RST> NAME <MYNAME> QTH <MYQTH>', fields)).toBe('DL1XYZ DE IU3ABC UR RST 599 NAME GIANNI QTH VERONA');
  });

  it('un dato vuoto non lascia spazi doppi', () => {
    expect(expandMacro('<CALL> DE <MYCALL> K', { ...fields, call: '' })).toBe('DE IU3ABC K');
  });

  it('una parola sconosciuta resta come stava', () => {
    expect(expandMacro('DE <MYCALL> <ALTRO>', fields)).toBe('DE IU3ABC <ALTRO>');
  });
});
