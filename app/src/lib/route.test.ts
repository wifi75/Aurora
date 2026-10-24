import { describe, expect, it } from 'vitest';
import { parseRoute } from './route';

describe('parseRoute', () => {
  it('reads the pairing link with the code', () => {
    expect(parseRoute('#/pair/12345678')).toEqual({ name: 'pair', code: '12345678' });
  });

  it('reads the pairing page without a code', () => {
    expect(parseRoute('#/pair')).toEqual({ name: 'pair', code: '' });
  });

  it('keeps only digits of the code', () => {
    expect(parseRoute('#/pair/12ab34')).toEqual({ name: 'pair', code: '1234' });
  });

  it('reads the settings and automation pages', () => {
    expect(parseRoute('#/impostazioni')).toEqual({ name: 'settings' });
    expect(parseRoute('#/automazioni')).toEqual({ name: 'automation' });
  });

  it('opens a precise settings tab', () => {
    expect(parseRoute('#/impostazioni/cluster')).toEqual({ name: 'settings', tab: 'cluster' });
    expect(parseRoute('#/impostazioni/')).toEqual({ name: 'live' });
  });

  it('reads the statistics and guide pages', () => {
    expect(parseRoute('#/statistiche')).toEqual({ name: 'stats' });
    expect(parseRoute('#/guida')).toEqual({ name: 'guide' });
  });

  it('reads the map and history pages', () => {
    expect(parseRoute('#/mappa')).toEqual({ name: 'map' });
    expect(parseRoute('#/storico')).toEqual({ name: 'history' });
  });

  it('defaults to the live page', () => {
    expect(parseRoute('')).toEqual({ name: 'live' });
    expect(parseRoute('#/')).toEqual({ name: 'live' });
    expect(parseRoute('#/sconosciuta')).toEqual({ name: 'live' });
  });
});
