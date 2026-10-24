import { beforeEach, describe, expect, it } from 'vitest';
import { addMacro, defaultMacros, expandMacro, readMacros, removeMacro, saveMacros, useMacros } from './macros';

function memory(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return { data, getItem: (k: string) => data[k] ?? null, setItem: (k: string, v: string) => { data[k] = v; } };
}
const broken = { getItem: () => { throw new Error('no'); }, setItem: () => { throw new Error('no'); } };

const ctx = { dx: 'BB2BB', me: 'AA1AA', report: '-10' };

describe('expandMacro', () => {
  it('replaces @ with the DX call, & with my call and # with the report', () => {
    expect(expandMacro('@ TNX 73', ctx)).toBe('BB2BB TNX 73');
    expect(expandMacro('@ &', ctx)).toBe('BB2BB AA1AA');
    expect(expandMacro('@ #', ctx)).toBe('BB2BB -10');
    expect(expandMacro('@ R#', ctx)).toBe('BB2BB R-10');
  });

  it('cuts the result to the 13 characters of a free message, uppercase', () => {
    expect(expandMacro('@ & TNX GL DX', ctx)).toBe('BB2BB AA1AA T');
    expect(expandMacro('ciao', ctx)).toBe('CIAO');
  });

  it('leaves out what is unknown instead of printing the symbol', () => {
    expect(expandMacro('@ TNX', { dx: '', me: 'AA1AA', report: '' })).toBe('TNX');
    expect(expandMacro('@ #', { dx: 'BB2BB', me: 'AA1AA', report: '' })).toBe('BB2BB');
  });
});

describe('the macro list', () => {
  beforeEach(() => localStorage.clear());

  it('starts with the five macros of JTDX', () => {
    expect(defaultMacros()).toEqual(['@ TNX 73', 'TU ^ 73', '@ &', '@ #', '@ R#']);
    expect(readMacros(memory())).toEqual(defaultMacros());
  });

  it('adds and removes without duplicates or empties', () => {
    expect(addMacro(['@ &'], ' 73 GL ')).toEqual(['@ &', '73 GL']);
    expect(addMacro(['@ &'], '@ &')).toEqual(['@ &']);
    expect(addMacro(['@ &'], '   ')).toEqual(['@ &']);
    expect(removeMacro(['@ &', '73'], '@ &')).toEqual(['73']);
  });

  it('is remembered and sanitised', () => {
    const store = memory();
    saveMacros(store, ['A', 'B']);
    expect(readMacros(store)).toEqual(['A', 'B']);
    expect(readMacros(memory({ 'aurora-macros': JSON.stringify(['ok', 3, '', 'x'.repeat(40)]) }))).toEqual(['ok']);
    expect(readMacros(memory({ 'aurora-macros': 'non json' }))).toEqual(defaultMacros());
    expect(readMacros(broken)).toEqual(defaultMacros());
    expect(() => saveMacros(broken, ['A'])).not.toThrow();
  });

  it('the shared store updates and persists at once', () => {
    useMacros.getState().add('NEW 73');
    expect(useMacros.getState().macros).toContain('NEW 73');
    expect(JSON.parse(localStorage.getItem('aurora-macros') ?? '[]')).toContain('NEW 73');
    useMacros.getState().remove('NEW 73');
    expect(useMacros.getState().macros).not.toContain('NEW 73');
    useMacros.getState().reset();
    expect(useMacros.getState().macros).toEqual(defaultMacros());
  });
});
