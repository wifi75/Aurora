import { describe, expect, it } from 'vitest';
import { contestBandColor, sortBands } from './contestBandColors';
import { multGridOf, type MultScore } from './contestMultGrid';

const score: MultScore = {
  griglia: [
    { tipo: 'dom', valore: 'CA', bande: ['10m', '15m'] },
    { tipo: 'dom', valore: 'NY', bande: ['10m'] },
    { tipo: 'dxcc', valore: 'DL', bande: [] },
  ],
  mancanti: [{ tipo: 'dom', per_banda: true, totali: 4, valori: ['TX', 'AK'] }],
  recenti: [{ nuovi: ['dom:NY'] }],
};

describe('griglia dei moltiplicatori', () => {
  it('una casella per ogni valore: lavorati con le bande e quelli che mancano in tono spento, in ordine', () => {
    const g = multGridOf(score, 'dom');
    expect(g.cells.map((c) => c.value)).toEqual(['AK', 'CA', 'NY', 'TX']);
    expect(g.cells.map((c) => c.worked)).toEqual([false, true, true, false]);
    expect(g.cells.find((c) => c.value === 'CA')?.bands).toEqual(['15m', '10m']);
    expect(g.worked).toBe(2);
    expect(g.total).toBe(4);
    expect(g.perBand).toBe(true);
  });

  it('il nuovo dell ultimo QSO e evidenziato', () => {
    const g = multGridOf(score, 'dom');
    expect(g.cells.find((c) => c.value === 'NY')?.fresh).toBe(true);
    expect(g.cells.find((c) => c.value === 'CA')?.fresh).toBe(false);
  });

  it('senza elenco completo ci sono solo i lavorati e nessun totale; un tipo a una volta sola non ha punti', () => {
    const g = multGridOf(score, 'dxcc');
    expect(g.cells.map((c) => c.value)).toEqual(['DL']);
    expect(g.total).toBeNull();
    expect(g.perBand).toBe(false);
  });

  it('il moltiplicatore del nominativo scritto compare pulsante, anche se l elenco non lo conosceva', () => {
    const g = multGridOf(score, 'dxcc', new Set(['dxcc:K', 'dom:TX']));
    expect(g.cells.find((c) => c.value === 'K')).toMatchObject({ worked: false, pending: true });
    expect(multGridOf(score, 'dom', new Set(['dom:TX'])).cells.find((c) => c.value === 'TX')?.pending).toBe(true);
  });

  it('i valori numerici si ordinano come numeri', () => {
    const s: MultScore = { griglia: ['10', '2', '1'].map((valore) => ({ tipo: 'cq', valore, bande: [] })), mancanti: [], recenti: [] };
    expect(multGridOf(s, 'cq').cells.map((c) => c.value)).toEqual(['1', '2', '10']);
  });
});

describe('colori delle bande', () => {
  it('quelli dei grafici dei programmi da gara e grigio per le bande sconosciute', () => {
    expect(contestBandColor('160m')).toBe('#e53935');
    expect(new Set(['160m', '80m', '40m', '20m', '15m', '10m'].map(contestBandColor)).size).toBe(6);
    expect(contestBandColor('xyz')).toBe('#90a4ae');
  });
  it('ordina dalla banda piu bassa', () => {
    expect(sortBands(['10m', '160m', '20m', '80m'])).toEqual(['160m', '80m', '20m', '10m']);
  });
});
