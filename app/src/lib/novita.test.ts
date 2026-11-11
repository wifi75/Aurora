import { describe, expect, it } from 'vitest';
import pkg from '../../package.json';
import { TABLE_LANGS } from '../i18n/core';
import { NOVITA, NOVITA_MAX } from './novita';

const version = pkg.version;

describe('novita', () => {
  it('ci sono al massimo 20 voci e la piu recente e della versione in uso: a ogni versione se ne aggiunge una', () => {
    expect(NOVITA.length).toBeLessThanOrEqual(NOVITA_MAX);
    expect(NOVITA.length).toBeGreaterThan(0);
    expect(NOVITA[0]?.version).toBe(version);
  });

  it('ogni voce ha titolo e testo in tutte e sei le lingue, e una data valida', () => {
    for (const n of NOVITA) {
      expect(n.title).toHaveLength(TABLE_LANGS.length);
      expect(n.text).toHaveLength(TABLE_LANGS.length);
      for (const s of [...n.title, ...n.text]) expect(s.trim().length).toBeGreaterThan(3);
      expect(Number.isNaN(Date.parse(n.released))).toBe(false);
    }
  });

  it('dalla piu recente alla piu vecchia (per versione)', () => {
    const key = (v: string) => v.split('.').map((x) => x.padStart(4, '0')).join('.');
    const versions = NOVITA.map((n) => key(n.version));
    expect(versions).toEqual([...versions].sort().reverse());
  });
});
