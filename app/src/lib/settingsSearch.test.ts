// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { collectEntries, fold, matchEntries } from './settingsSearch';

describe('fold', () => {
  it('toglie maiuscole e accenti', () => {
    expect(fold('Già Lavorati')).toBe('gia lavorati');
  });
});

describe('collectEntries', () => {
  it('legge le etichette, i titoli e i campi con aria-label, senza doppioni', () => {
    const root = document.createElement('div');
    root.innerHTML = `
      <h3>Già lavorati</h3>
      <label><input type="checkbox"> Paese lavorato ma non confermato = DXCC nuovo</label>
      <label>Paese lavorato ma non confermato = DXCC nuovo</label>
      <input aria-label="Frequenza di ricezione">
      <label>Sì</label>`;
    const found = collectEntries('colors', root).map((e) => e.text);
    expect(found).toEqual(['Già lavorati', 'Paese lavorato ma non confermato = DXCC nuovo', 'Frequenza di ricezione']);
  });
});

describe('matchEntries', () => {
  const entries = [
    { tab: 'colors', text: 'Paese lavorato ma non confermato = DXCC nuovo' },
    { tab: 'colors', text: 'Già lavorati' },
    { tab: 'radio', text: 'Porta seriale' },
  ];

  it('cerca senza badare ad accenti e maiuscole, con tutte le parole', () => {
    expect(matchEntries(entries, 'gia lav').map((e) => e.text)).toEqual(['Già lavorati']);
    expect(matchEntries(entries, 'DXCC confermato').map((e) => e.tab)).toEqual(['colors']);
  });

  it('una ricerca vuota o senza risultati non dà niente', () => {
    expect(matchEntries(entries, '   ')).toEqual([]);
    expect(matchEntries(entries, 'zzz')).toEqual([]);
  });
});
