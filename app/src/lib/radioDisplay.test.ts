import { describe, expect, it } from 'vitest';
import {
  alcFraction,
  compFraction,
  DEFAULT_RADIO_DISPLAY,
  displayModel,
  formatRadioFrequency,
  hasFrequency,
  litCount,
  mainMeter,
  meterReadouts,
  powerFraction,
  readRadioDisplay,
  saveRadioDisplay,
  S_TICKS,
  sFraction,
  sLabel,
  swrFraction,
} from './radioDisplay';

describe('formatRadioFrequency', () => {
  it('writes MHz.kHz.tens of Hz like a radio display', () => {
    expect(formatRadioFrequency(21_074_000)).toBe('21.074.00');
    expect(formatRadioFrequency(14_074_550)).toBe('14.074.55');
    expect(formatRadioFrequency(7_074_000)).toBe('7.074.00');
    expect(formatRadioFrequency(144_174_000)).toBe('144.174.00');
  });

  it('can show whole hertz', () => {
    expect(formatRadioFrequency(21_074_000, 'hz')).toBe('21.074.000');
    expect(formatRadioFrequency(3_573_005, 'hz')).toBe('3.573.005');
  });

  it('shows dashes when the frequency is zero or unknown', () => {
    expect(formatRadioFrequency(0)).toBe('--.---.--');
    expect(formatRadioFrequency(Number.NaN, 'hz')).toBe('--.---.---');
    expect(hasFrequency(0)).toBe(false);
    expect(hasFrequency(undefined)).toBe(false);
    expect(hasFrequency(7_074_000)).toBe(true);
  });
});

describe('meter scales', () => {
  it('puts S9 at 60 percent and counts decibels above it', () => {
    expect(sFraction({ s_points: 0 })).toBe(0);
    expect(sFraction({ s_points: 9 })).toBeCloseTo(0.6);
    expect(sFraction({ s_points: 9, s_over_db: 60 })).toBe(1);
    expect(sFraction({ s_points: 9, s_over_db: 30 })).toBeCloseTo(0.8);
    expect(sFraction({ s_points: 4.5 })).toBeCloseTo(0.3);
  });

  it('turns the S scale off without data instead of inventing a value', () => {
    expect(sFraction(null)).toBeNull();
    expect(sFraction(undefined)).toBeNull();
    expect(sFraction({ power_w: 5 })).toBeNull();
    expect(sLabel({ power_w: 5 })).toBeNull();
  });

  it('writes the S reading', () => {
    expect(sLabel({ s_points: 7 })).toBe('S7');
    expect(sLabel({ s_points: 9, s_over_db: 20 })).toBe('S9+20');
    expect(sLabel({ s_over_db: 10 })).toBe('S9+10');
    expect(sLabel({ s_points: 9 })).toBe('S9');
  });

  it('maps power, ALC and compression linearly, clamped', () => {
    expect(powerFraction({ power_w: 50 })).toBe(0.5);
    expect(powerFraction({ power_w: 250 })).toBe(1);
    expect(powerFraction({})).toBeNull();
    expect(alcFraction({ alc: 0.25 })).toBe(0.25);
    expect(alcFraction({ alc: 3 })).toBe(1);
    expect(alcFraction({ alc: -1 })).toBe(0);
    expect(compFraction({ comp_db: 15 })).toBe(0.5);
    expect(compFraction({})).toBeNull();
  });

  it('maps the SWR on its non-linear scale', () => {
    expect(swrFraction({ swr: 1 })).toBe(0);
    expect(swrFraction({ swr: 0.9 })).toBe(0);
    expect(swrFraction({ swr: 1.5 })).toBeCloseTo(0.25);
    expect(swrFraction({ swr: 2 })).toBeCloseTo(0.5);
    expect(swrFraction({ swr: 2.5 })).toBeCloseTo(0.625);
    expect(swrFraction({ swr: 3 })).toBeCloseTo(0.75);
    expect(swrFraction({ swr: 99 })).toBe(1);
    expect(swrFraction({})).toBeNull();
  });

  it('counts the lit segments, none when the scale is off', () => {
    expect(litCount(null, 20)).toBe(0);
    expect(litCount(0.5, 20)).toBe(10);
    expect(litCount(2, 20)).toBe(20);
  });

  it('has S ticks in increasing order ending at the full scale', () => {
    const fractions = S_TICKS.map((t) => t.fraction);
    expect([...fractions].sort((a, b) => a - b)).toEqual(fractions);
    expect(S_TICKS.find((t) => t.label === '9')?.fraction).toBeCloseTo(0.6);
    expect(fractions[fractions.length - 1]).toBe(1);
  });
});

describe('main meter and readouts', () => {
  const base = { dial_hz: 14_074_000, mode: 'FT8', transmitting: false, decoding: true, tx_enabled: false };

  it('reads the S-meter while receiving and the power while transmitting', () => {
    const meters = { s_points: 5, power_w: 20 };
    expect(mainMeter(displayModel({ ...base, meters }))).toMatchObject({ kind: 'S', label: 'S5' });
    expect(mainMeter(displayModel({ ...base, transmitting: true, meters }))).toMatchObject({ kind: 'PO', label: '20 W', fraction: 0.2 });
  });

  it('leaves the meter off when the radio does not give it', () => {
    expect(mainMeter(displayModel({ ...base }))).toEqual({ kind: 'S', fraction: null, label: null });
    expect(mainMeter(displayModel({ ...base, transmitting: true, meters: { s_points: 5 } }))).toEqual({ kind: 'PO', fraction: null, label: null });
  });

  it('lists only the readings that exist', () => {
    expect(meterReadouts(null)).toEqual([]);
    expect(meterReadouts({ s_points: 3 })).toEqual([]);
    expect(meterReadouts({ power_w: 5, swr: 1.25, alc: 0.4, comp_db: 6, id_a: 3.21, vdd_v: 13.8 })).toEqual([
      { key: 'PO', text: '5 W' },
      { key: 'SWR', text: '1.3' },
      { key: 'ALC', text: '40 %' },
      { key: 'COMP', text: '6 dB' },
      { key: 'Id', text: '3.2 A' },
      { key: 'Vdd', text: '13.8 V' },
    ]);
  });

  it('builds the model with safe defaults when there is no status', () => {
    expect(displayModel(null)).toEqual({ dialHz: 0, mode: '', transmitting: false, decoding: false, txEnabled: false, meters: null });
  });
});

describe('saved style', () => {
  const store = (raw: string | null) => ({ getItem: () => raw });

  it('defaults to the fluorescent style, shown', () => {
    expect(readRadioDisplay(store(null))).toEqual(DEFAULT_RADIO_DISPLAY);
    expect(readRadioDisplay(null)).toEqual(DEFAULT_RADIO_DISPLAY);
  });

  it('round-trips the choice under the aurora-radio-display key', () => {
    const saved: Record<string, string> = {};
    saveRadioDisplay({ setItem: (k, v) => void (saved[k] = v) }, { style: 'amber', hidden: true });
    expect(Object.keys(saved)).toEqual(['aurora-radio-display']);
    expect(readRadioDisplay(store(saved['aurora-radio-display'] ?? null))).toEqual({ style: 'amber', hidden: true });
  });

  it('falls back on garbage or unknown styles', () => {
    expect(readRadioDisplay(store('non json'))).toEqual(DEFAULT_RADIO_DISPLAY);
    expect(readRadioDisplay(store('{"style":"nixie","hidden":true}'))).toEqual({ style: 'fluorescent', hidden: true });
    expect(readRadioDisplay({ getItem: () => { throw new Error('negato'); } })).toEqual(DEFAULT_RADIO_DISPLAY);
  });
});
