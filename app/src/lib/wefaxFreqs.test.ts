import { beforeEach, describe, expect, it } from 'vitest';
import {
  EMPTY_STATE,
  WEFAX_FREQ_KEY,
  effectivePresets,
  normalizeEntry,
  parseWefaxFreq,
  readWefaxFreq,
  removeEntry,
  resetWefaxFreq,
  saveEntry,
  validateEntry,
  writeWefaxFreq,
} from './wefaxFreqs';
import { wefaxGroups, wefaxPresets } from './wefaxStations';

const factory = wefaxPresets();
const ddk3 = factory.find((p) => p.khz === 7880);
const ddk3Id = ddk3?.id ?? '';

beforeEach(() => localStorage.clear());

describe('frequenze WEFAX dell utente', () => {
  it('senza nulla salvato vale l elenco di fabbrica', () => {
    expect(readWefaxFreq()).toEqual(EMPTY_STATE);
    expect(effectivePresets(readWefaxFreq())).toEqual(factory);
  });

  it('unisce fabbrica e voci aggiunte, con la portante 1,9 kHz sotto', () => {
    const s = saveEntry(EMPTY_STATE, null, { call: 'IU3ABC', place: 'Test', khz: 6000 });
    writeWefaxFreq(s);
    const list = effectivePresets(readWefaxFreq());
    expect(list).toHaveLength(factory.length + 1);
    const mine = list.find((p) => p.call === 'IU3ABC');
    expect(mine?.dialHz).toBe(5_998_100);
    expect(mine?.band).toBe(6);
    expect(mine?.id.startsWith('u:')).toBe(true);
  });

  it('cancellare una voce di fabbrica la nasconde; il ripristino la riporta', () => {
    writeWefaxFreq(removeEntry(EMPTY_STATE, ddk3Id));
    expect(readWefaxFreq().hidden).toEqual([ddk3Id]);
    expect(effectivePresets(readWefaxFreq()).some((p) => p.id === ddk3Id)).toBe(false);
    resetWefaxFreq();
    expect(localStorage.getItem(WEFAX_FREQ_KEY)).toBeNull();
    expect(effectivePresets(readWefaxFreq())).toEqual(factory);
  });

  it('cambiare una voce di fabbrica tiene il suo id e i suoi IOC/righe; cancellare una aggiunta la toglie', () => {
    let s = saveEntry(EMPTY_STATE, ddk3Id, { call: 'DDK3', place: 'Pinneberg (DE)', khz: 7881, band: 'Meteo DE' });
    let p = effectivePresets(s).find((x) => x.id === ddk3Id);
    expect(p?.khz).toBe(7881);
    expect(p?.dialHz).toBe(7_879_100);
    expect(p?.bandLabel).toBe('Meteo DE');
    expect(p?.ioc).toBe(ddk3?.ioc);
    s = saveEntry(s, null, { call: 'IU3ABC', place: '', khz: 500 });
    const added = s.added[0]?.id ?? '';
    s = removeEntry(s, added);
    expect(s.added).toEqual([]);
    s = removeEntry(s, ddk3Id);
    expect(s.edited).toEqual({});
    p = effectivePresets(s).find((x) => x.id === ddk3Id);
    expect(p).toBeUndefined();
  });

  it('una banda nuova fa un gruppo suo nel menu; senza etichetta si ricava dalla frequenza', () => {
    const s = saveEntry(EMPTY_STATE, null, { call: 'IU3ABC', place: 'Test', khz: 7900, band: 'Mie' });
    const groups = wefaxGroups(effectivePresets(s));
    expect(groups.find((g) => g.label === 'Mie')?.presets.map((p) => p.call)).toEqual(['IU3ABC']);
    expect(groups.find((g) => !g.label && g.band === 7)?.presets.some((p) => p.call === 'IU3ABC')).toBe(false);
  });

  it('valida frequenza, nominativo, luogo, banda e doppioni', () => {
    const list = effectivePresets(EMPTY_STATE);
    const ok = { call: 'IU3ABC', place: 'Test', khz: '7000,5', band: '' };
    expect(validateEntry(ok, list)).toBeNull();
    expect(normalizeEntry(ok)).toEqual({ call: 'IU3ABC', place: 'Test', khz: 7000.5 });
    expect(validateEntry({ ...ok, khz: '' }, list)).toBe('khz');
    expect(validateEntry({ ...ok, khz: 'abc' }, list)).toBe('khz');
    expect(validateEntry({ ...ok, khz: '-5' }, list)).toBe('khz');
    expect(validateEntry({ ...ok, khz: '99' }, list)).toBe('khz');
    expect(validateEntry({ ...ok, khz: '30001' }, list)).toBe('khz');
    expect(validateEntry({ ...ok, call: '  ' }, list)).toBe('call');
    expect(validateEntry({ ...ok, call: 'X'.repeat(25) }, list)).toBe('call');
    expect(validateEntry({ ...ok, place: 'X'.repeat(41) }, list)).toBe('place');
    expect(validateEntry({ ...ok, band: 'X'.repeat(17) }, list)).toBe('band');
    expect(validateEntry({ ...ok, call: 'ddk3', khz: '7880' }, list)).toBe('dup');
    // la voce che si sta cambiando non e' un doppione di se stessa; stessa frequenza con altro nominativo va bene
    expect(validateEntry({ ...ok, call: 'DDK3', khz: '7880' }, list, ddk3Id)).toBeNull();
    expect(validateEntry({ ...ok, call: 'IU3ABC', khz: '7880' }, list)).toBeNull();
  });

  it('resiste a dati rotti o malformati', () => {
    expect(parseWefaxFreq('{rotto')).toEqual(EMPTY_STATE);
    expect(parseWefaxFreq('null')).toEqual(EMPTY_STATE);
    expect(parseWefaxFreq('[]')).toEqual(EMPTY_STATE);
    expect(parseWefaxFreq(JSON.stringify({ v: 99, hidden: [ddk3Id] }))).toEqual(EMPTY_STATE);
    const s = parseWefaxFreq(
      JSON.stringify({
        v: 1,
        hidden: [ddk3Id, ddk3Id, 'non-esiste', 7],
        edited: { [ddk3Id]: { call: 'DDK3', place: 'x', khz: 'tanta' }, altro: { call: 'A', place: '', khz: 5000 } },
        added: [
          { id: 'u:1', call: 'IU3ABC', place: 'Test', khz: 5000 },
          { id: 'u:1', call: 'IU3ABC', place: 'Doppio id', khz: 5100 },
          { id: 'u:2', call: '', place: '', khz: 5000 },
          { id: 'u:3', call: 'IU3ABC', place: '', khz: 50_000 },
          { id: 'x', call: 'IU3ABC', place: '', khz: 5200 },
          'testo',
          null,
        ],
      }),
    );
    expect(s.hidden).toEqual([ddk3Id]);
    expect(s.edited).toEqual({});
    expect(s.added).toEqual([{ id: 'u:1', call: 'IU3ABC', place: 'Test', khz: 5000 }]);
    localStorage.setItem(WEFAX_FREQ_KEY, '{"v":1,"added":"no"}');
    expect(readWefaxFreq()).toEqual(EMPTY_STATE);
  });

  it('salva JSON versionato nella chiave aurora-wefax-freq', () => {
    writeWefaxFreq(removeEntry(EMPTY_STATE, ddk3Id));
    expect(JSON.parse(localStorage.getItem(WEFAX_FREQ_KEY) ?? '{}')).toMatchObject({ v: 1, hidden: [ddk3Id] });
    // senza differenze la chiave si toglie
    writeWefaxFreq(EMPTY_STATE);
    expect(localStorage.getItem(WEFAX_FREQ_KEY)).toBeNull();
  });
});
