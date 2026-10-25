import { describe, expect, it } from 'vitest';
import { bandHeader, cellKind, dxccQueryString, filterCountries, subjectBand, subjectMode, totalKind, type DxccCountry } from './dxccCard';

const countries: DxccCountry[] = [
  { id: 1, name: 'Atlantis', prefix: 'AT', continent: 'EU', cells: { FT8: { '20m': 'worked' }, ALL: { '20m': 'worked', '40m': 'unconfirmed' } } },
  { id: 2, name: 'Lemuria', prefix: 'LM', continent: 'NA', cells: { ALL: { '20m': 'worked' } } },
];

describe('dxccCard', () => {
  it('builds the query string without the empty values', () => {
    expect(dxccQueryString({})).toBe('');
    expect(dxccQueryString({ call: ' K1ABC ', grid: '', band: '20m', mode: 'FT8', compact: true })).toBe('?call=K1ABC&band=20m&mode=FT8&compact=1');
  });

  it('takes band and mode from the radio', () => {
    expect(subjectBand(14_074_000)).toBe('20m');
    expect(subjectBand(0)).toBeUndefined();
    expect(subjectBand(null)).toBeUndefined();
    expect(subjectMode('ft8')).toBe('FT8');
    expect(subjectMode(' ')).toBeUndefined();
  });

  it('names the colour of a cell and of a total', () => {
    expect(cellKind(undefined)).toBe('none');
    expect(cellKind('unconfirmed')).toBe('unconfirmed');
    expect(totalKind({ countries: 0, unconfirmed: 0, state: 'worked' })).toBe('none');
    expect(totalKind({ countries: 4, unconfirmed: 1, state: 'unconfirmed' })).toBe('unconfirmed');
    expect(totalKind(undefined)).toBe('none');
  });

  it('shortens the band headers', () => {
    expect(bandHeader('160m')).toBe('160');
    expect(bandHeader('U')).toBe('U');
  });

  it('filters the countries by text and by mode row', () => {
    expect(filterCountries(countries, '', 'ALL')).toHaveLength(2);
    expect(filterCountries(countries, 'lem', 'ALL').map((c) => c.name)).toEqual(['Lemuria']);
    expect(filterCountries(countries, 'at', 'ALL').map((c) => c.name)).toEqual(['Atlantis']);
    expect(filterCountries(countries, '', 'FT8').map((c) => c.name)).toEqual(['Atlantis']);
    expect(filterCountries(countries, '', 'DIG')).toEqual([]);
  });
});
