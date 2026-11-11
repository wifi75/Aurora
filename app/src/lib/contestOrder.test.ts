import { describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import type { Valore } from '../api/contest';
import { chiamabile, decodificheVisibili, ordinaRighe, peso } from './contestOrder';

const dec = (time_ms: number, snr = 0) => ({ time_ms, snr }) as DecodedMessage;
const val = (punti: number, extra: Partial<Valore> = {}) => ({ punti, molt_nuovi: [], doppio: false, fuori_regola: false, ...extra }) as Valore;

describe('decodificheVisibili', () => {
  const t0 = 1_700_000_000_000 - (1_700_000_000_000 % 15_000);
  it('Valore: tiene solo il periodo corrente e il precedente (FT8)', () => {
    const tutte = [dec(t0), dec(t0 - 15_000), dec(t0 - 30_000), dec(t0 - 600_000)];
    expect(decodificheVisibili(tutte, 'valore', 'FT8').map((d) => d.time_ms)).toEqual([t0, t0 - 15_000]);
  });
  it('Valore: in FT4 i periodi durano 7,5 s', () => {
    const f4 = 1_700_000_000_000 - (1_700_000_000_000 % 7_500);
    const tutte = [dec(f4), dec(f4 - 7_500), dec(f4 - 15_000)];
    expect(decodificheVisibili(tutte, 'valore', 'FT4')).toHaveLength(2);
  });
  it('Ora: nessun filtro', () => {
    const tutte = [dec(t0), dec(t0 - 600_000)];
    expect(decodificheVisibili(tutte, 'ora', 'FT8')).toHaveLength(2);
  });
  it('senza decodifiche non fa nulla', () => {
    expect(decodificheVisibili([], 'valore', 'FT8')).toEqual([]);
  });
});

describe('ordinaRighe', () => {
  const righe = [
    { id: 'a', d: dec(200, -5), v: val(1) },
    { id: 'b', d: dec(100, -10), v: val(5) },
    { id: 'c', d: dec(300, 3), v: val(5, { doppio: true }) },
    { id: 'd', d: dec(250, 2), v: val(5) },
  ];
  it('Valore: il piu\' utile in cima, a parita\' il segnale piu\' forte, i doppi in fondo', () => {
    expect(ordinaRighe(righe, 'valore').map((r) => r.id)).toEqual(['d', 'b', 'a', 'c']);
  });
  it('Ora: dalla piu\' vecchia, la nuova in fondo', () => {
    expect(ordinaRighe(righe, 'ora').map((r) => r.id)).toEqual(['b', 'a', 'd', 'c']);
  });
  it('non modifica l\'elenco ricevuto', () => {
    const copia = [...righe];
    ordinaRighe(righe, 'ora');
    expect(righe).toEqual(copia);
  });
  it('Valore: i CQ e le chiamate a me prima dei QSO fra altre stazioni', () => {
    const msg = (id: string, text: string, punti: number) => ({ id, d: { ...dec(100), text }, v: val(punti) });
    const r = [msg('altri', 'K1ABC W9XYZ R-05', 50), msg('cq', 'CQ TEST DL1AAA JO62', 2), msg('me', 'IU3ABC F5HHH JN18', 1)];
    expect(ordinaRighe(r, 'valore', 'IU3ABC').map((x) => x.id)).toEqual(['cq', 'me', 'altri']);
  });
  it('chiamabile: CQ, QRZ e chi chiama me', () => {
    expect(chiamabile('CQ WW K1ABC FN42', 'IU3ABC')).toBe(true);
    expect(chiamabile('QRZ K1ABC', '')).toBe(true);
    expect(chiamabile('<IU3ABC> K1ABC R-10', 'iu3abc')).toBe(true);
    expect(chiamabile('W9XYZ K1ABC RR73', 'IU3ABC')).toBe(false);
  });
  it('peso: senza valutazione 0', () => {
    expect(peso(undefined)).toBe(0);
  });
});
