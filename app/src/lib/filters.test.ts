import { beforeEach, describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { applyFilters, callOf, defaultFilters, readFilters, saveFilters, useFilters, type Filters } from './filters';

const d = (id: number, text: string, over: Partial<DecodedMessage> = {}): DecodedMessage => ({
  id, time_ms: id, snr: -10, delta_time: 0, freq_hz: 1500, text, tag: null, country: null, continent: null, ...over,
});

const sample = [
  d(1, 'CQ DX JA1XYZ PM95', { country: 'Japan', continent: 'AS' }),
  d(2, 'AA1AA BB2BB -10', { country: 'Italy', continent: 'EU' }),
  d(3, 'BB2BB AA1AA RR73', { country: 'Italy', continent: 'EU' }),
  d(4, 'CC3CC DD4DD 73', { country: 'Germany', continent: 'EU' }),
  d(5, '<...> EE5EE KK00', { country: 'Spain', continent: 'EU' }),
  d(6, 'TNX 73 GL', {}),
  d(7, 'CQ W1ABC FN42', { country: 'United States', continent: 'NA' }),
];
const ids = (list: DecodedMessage[]) => list.map((x) => x.id);
const f = (over: Partial<Filters>): Filters => ({ ...defaultFilters(), ...over });

function memory(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return { data, getItem: (k: string) => data[k] ?? null, setItem: (k: string, v: string) => { data[k] = v; } };
}
const broken = { getItem: () => { throw new Error('no'); }, setItem: () => { throw new Error('no'); } };

describe('callOf', () => {
  it('reads the sender of a message', () => {
    expect(callOf('CQ DX JA1XYZ PM95')).toBe('JA1XYZ');
    expect(callOf('AA1AA BB2BB -10')).toBe('BB2BB');
    expect(callOf('<...> EE5EE KK00')).toBe('EE5EE');
    expect(callOf('TNX 73 GL')).toBeNull();
  });
});

describe('applyFilters', () => {
  it('lets everything through by default', () => {
    expect(ids(applyFilters(sample, defaultFilters(), 'EU'))).toEqual([1, 2, 3, 4, 5, 6, 7]);
  });

  it('hides the chosen continents', () => {
    expect(ids(applyFilters(sample, f({ hideContinents: ['AS', 'NA'] }), 'EU'))).toEqual([2, 3, 4, 5, 6]);
  });

  it('hides the chosen countries, ignoring case', () => {
    expect(ids(applyFilters(sample, f({ hideCountries: ['japan', 'ITALY'] }), 'EU'))).toEqual([4, 5, 6, 7]);
  });

  it('hides the chosen callsigns, with a trailing * as a prefix', () => {
    // si nascondono i messaggi di chi trasmette: il 3 e' di AA1AA verso BB2BB e resta
    expect(ids(applyFilters(sample, f({ hideCalls: ['bb2bb'] }), 'EU'))).toEqual([1, 3, 4, 5, 6, 7]);
    expect(ids(applyFilters(sample, f({ hideCalls: ['JA*', 'W1*'] }), 'EU'))).toEqual([2, 3, 4, 5, 6]);
  });

  it('shows only CQ', () => {
    expect(ids(applyFilters(sample, f({ only: 'cq' }), 'EU'))).toEqual([1, 7]);
  });

  it('shows CQ, RRR, RR73 and 73, or only CQ, RR73 and 73', () => {
    expect(ids(applyFilters(sample, f({ only: 'cq_rr73' }), 'EU'))).toEqual([1, 3, 4, 7]);
    const rrr = [d(1, 'AA1AA BB2BB RRR'), d(2, 'AA1AA BB2BB -10')];
    expect(ids(applyFilters(rrr, f({ only: 'cq_rrr' }), 'EU'))).toEqual([1]);
    expect(ids(applyFilters(rrr, f({ only: 'cq_rr73' }), 'EU'))).toEqual([]);
  });

  it('hides non-standard messages', () => {
    expect(ids(applyFilters(sample, f({ hideNonStandard: true }), 'EU'))).toEqual([1, 2, 3, 4, 7]);
  });

  it('hides the messages of the own continent, only when it is known', () => {
    expect(ids(applyFilters(sample, f({ hideOwnContinent: true }), 'EU'))).toEqual([1, 6, 7]);
    expect(ids(applyFilters(sample, f({ hideOwnContinent: true }), null))).toEqual([1, 2, 3, 4, 5, 6, 7]);
  });

  it('hides the CQ of stations already worked on the band but keeps the rest of their traffic', () => {
    const list = [
      d(1, 'CQ K1ABC FN42', { tag: 'worked_call' }),
      d(2, 'AA1AA K1ABC -10', { tag: 'worked_call' }),
      d(3, 'CQ W1XYZ FN31', { tag: 'new_dxcc' }),
      d(4, 'CQ DL1AB JO62', { tag: null }),
    ];
    expect(ids(applyFilters(list, f({ hideWorked: true }), 'EU'))).toEqual([2, 3, 4]);
    expect(ids(applyFilters(list, f({ hideWorked: false }), 'EU'))).toEqual([1, 2, 3, 4]);
  });

  it('combines the filters', () => {
    expect(ids(applyFilters(sample, f({ only: 'cq', hideContinents: ['NA'] }), 'EU'))).toEqual([1]);
  });
});

describe('remembered filters', () => {
  beforeEach(() => localStorage.clear());

  it('start empty', () => {
    expect(readFilters(memory())).toEqual(defaultFilters());
    expect(readFilters(null)).toEqual(defaultFilters());
  });

  it('are remembered and sanitised', () => {
    const store = memory();
    saveFilters(store, f({ hideContinents: ['AS'], hideCountries: ['Japan'], hideCalls: ['AA1AA'], only: 'cq', hideNonStandard: true }));
    expect(readFilters(store)).toEqual(f({ hideContinents: ['AS'], hideCountries: ['Japan'], hideCalls: ['AA1AA'], only: 'cq', hideNonStandard: true }));
    const raw = '{"hideContinents":["AS","XX",5],"hideCountries":[1,"Spain"],"hideCalls":"no","only":"boh","hideNonStandard":"si"}';
    expect(readFilters(memory({ 'aurora-filters': raw }))).toEqual(f({ hideContinents: ['AS'], hideCountries: ['Spain'] }));
    expect(readFilters(memory({ 'aurora-filters': 'non json' }))).toEqual(defaultFilters());
    expect(readFilters(broken)).toEqual(defaultFilters());
    expect(() => saveFilters(broken, defaultFilters())).not.toThrow();
  });

  it('the shared store updates and persists at once', () => {
    useFilters.getState().set({ only: 'cq' });
    expect(useFilters.getState().filters.only).toBe('cq');
    expect(JSON.parse(localStorage.getItem('aurora-filters') ?? '{}').only).toBe('cq');
    useFilters.getState().reset();
    expect(useFilters.getState().filters).toEqual(defaultFilters());
  });
});
