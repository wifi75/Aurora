import { describe, expect, it } from 'vitest';
import { bandPlanRows } from './bandplan';

// i tratti dei digimodi a banda stretta (massimo 500 Hz) del piano IARU Regione 1 in vigore dal 16/10/2020, in Hz
const TRATTI: Record<string, [number, number]> = {
  '160m': [1_838_000, 1_840_000],
  '80m': [3_580_000, 3_590_000],
  '40m': [7_040_000, 7_047_000],
  '30m': [10_130_000, 10_150_000],
  '20m': [14_070_000, 14_089_000],
  '17m': [18_095_000, 18_105_000],
  '15m': [21_070_000, 21_090_000],
  '12m': [24_915_000, 24_925_000],
  '10m': [28_070_000, 28_120_000],
};

describe('piano di banda di Olivia e Contestia', () => {
  it('le frequenze di Regione 1 stanno nei tratti dei digimodi con 500 Hz di banda a 1500 Hz di audio', () => {
    const righe = bandPlanRows('OLIVIA').filter((r) => r.region === 'ALL' || r.region === 'R1');
    expect(righe.length).toBe(Object.keys(TRATTI).length);
    for (const r of righe) {
      const [da, a] = TRATTI[r.band] ?? [0, 0];
      expect(r.hz).toBeGreaterThanOrEqual(da);
      expect(r.hz + 1_750).toBeLessThanOrEqual(a);
    }
  });
});
