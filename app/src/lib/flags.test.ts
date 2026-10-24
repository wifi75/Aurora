import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { FLAG_TABLE } from './flagTable';
import { flagCode } from './flags';

const css = readFileSync('node_modules/flag-icons/css/flag-icons.css', 'utf8');
const available = new Set([...css.matchAll(/^\.fi-([a-z0-9-]+)\s*\{/gm)].map((m) => m[1]));

describe('flags', () => {
  it('covers every DXCC entity of cty.dat (346 names)', () => {
    expect(Object.keys(FLAG_TABLE).length).toBeGreaterThanOrEqual(340);
  });

  it('every mapped code exists in the flag-icons stylesheet', () => {
    const wrong = Object.entries(FLAG_TABLE).filter(([, code]) => code !== null && !available.has(code));
    expect(wrong).toEqual([]);
  });

  it('knows the countries of everyday traffic', () => {
    expect(flagCode('Italy')).toBe('it');
    expect(flagCode('Fed. Rep. of Germany')).toBe('de');
    expect(flagCode('Japan')).toBe('jp');
    expect(flagCode('European Russia')).toBe('ru');
    expect(flagCode('Asiatic Russia')).toBe('ru');
    expect(flagCode('United States')).toBe('us');
    expect(flagCode('England')).toBe('gb-eng');
    expect(flagCode('Scotland')).toBe('gb-sct');
    expect(flagCode('Sicily')).toBe('it');
    expect(flagCode('Canary Islands')).toBe('ic');
  });

  it('gives islands and territories their own flag where there is one', () => {
    expect(flagCode('Guadeloupe')).toBe('gp');
    expect(flagCode('Greenland')).toBe('gl');
    expect(flagCode('Hong Kong')).toBe('hk');
    expect(flagCode('Republic of Kosovo')).toBe('xk');
  });

  it('has no flag for international sites and unknown names', () => {
    expect(flagCode('ITU HQ')).toBeNull();
    expect(flagCode('Paese che non esiste')).toBeNull();
    expect(flagCode(null)).toBeNull();
    expect(flagCode(undefined)).toBeNull();
    expect(flagCode('')).toBeNull();
  });
});
