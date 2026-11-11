import { describe, expect, it } from 'vitest';
import type { FrequencyEntry } from '../api/client';
import { targetDial } from './modeRetune';

const row = (band: string, mode: string, hz: number, region: FrequencyEntry['region'] = 'ALL', preferred = true): FrequencyEntry => ({ band, mode, hz, region, preferred });
const MSK: FrequencyEntry[] = [row('6m', 'MSK144', 50_260_000, 'R2'), row('6m', 'MSK144', 50_260_000, 'R3'), row('6m', 'MSK144', 50_380_000, 'R1'), row('2m', 'MSK144', 144_150_000)];
const FT8: FrequencyEntry[] = [row('20m', 'FT8', 14_074_000), row('15m', 'FT8', 21_074_000), row('6m', 'FT8', 50_313_000), row('2m', 'FT8', 144_174_000)];

describe('targetDial', () => {
  it('MSK144 dai 15 m va ai 6 m, sulla frequenza della Regione scelta', () => {
    expect(targetDial(MSK, 'R1', 21_074_000)).toBe(50_380_000);
    expect(targetDial(MSK, 'R2', 21_074_000)).toBe(50_260_000);
    expect(targetDial(MSK, 'R3', 21_074_000)).toBe(50_260_000);
    // con «tutte» la prima per frequenza
    expect(targetDial(MSK, 'ALL', 21_074_000)).toBe(50_260_000);
  });

  it('se la banda di adesso ha il modo si resta sulla banda', () => {
    // da MSK144 (6 m) a FT8: la banda e la stessa
    expect(targetDial(FT8, 'R1', 50_380_000)).toBe(50_313_000);
    // FT8 -> FT4 sui 20 m
    expect(targetDial([row('20m', 'FT4', 14_080_000)], 'ALL', 14_074_000)).toBe(14_080_000);
  });

  it('gia su una frequenza del modo non si muove', () => {
    expect(targetDial(MSK, 'R1', 50_380_000)).toBeNull();
    expect(targetDial(FT8, 'ALL', 14_074_000)).toBeNull();
  });

  it('senza righe o senza frequenza della radio non si muove', () => {
    expect(targetDial([], 'ALL', 14_074_000)).toBeNull();
    expect(targetDial(MSK, 'ALL', 0)).toBeNull();
  });

  it('con una regione senza righe del modo si usano tutte', () => {
    expect(targetDial([row('6m', 'MSK144', 50_380_000, 'R1')], 'R2', 21_074_000)).toBe(50_380_000);
  });
});
