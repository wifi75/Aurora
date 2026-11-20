import { describe, expect, it } from 'vitest';
import { clampStep, TRACKING_DEFAULTS } from './SatTrackingPane';

describe('clampStep', () => {
  it('porta il valore dentro i limiti e sul passo', () => {
    expect(clampStep(700, 0, 600, 5)).toBe(600);
    expect(clampStep(-3, 0, 600, 5)).toBe(0);
    expect(clampStep(62, 0, 600, 5)).toBe(60);
    expect(clampStep(2.3, 0, 10, 0.5)).toBe(2.5);
    expect(clampStep(Number.NaN, 1, 10, 1)).toBe(1);
  });

  it('i valori di fabbrica sono prudenti: nessun parcheggio, anticipo 60 s', () => {
    expect(TRACKING_DEFAULTS.rotor_after).toBe('stay');
    expect(TRACKING_DEFAULTS.rotor_lead_s).toBe(60);
    expect(TRACKING_DEFAULTS.rotor_interval_s).toBe(2);
    expect(TRACKING_DEFAULTS.rotor_min_move_deg).toBe(2);
    expect(TRACKING_DEFAULTS.rotor_after_delay_s).toBe(10);
  });
});
