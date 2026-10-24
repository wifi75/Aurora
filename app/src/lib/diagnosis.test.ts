import { describe, expect, it } from 'vitest';
import type { StationStatus } from '../api/schemas';
import { diagnose, type DiagnosisInput } from './diagnosis';

const status = (over: Partial<StationStatus> = {}): StationStatus => ({ dial_hz: 14_074_000, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: false, de_call: 'IU3ABC', de_grid: 'IO91wm', ...over });
const base = (over: Partial<DiagnosisInput> = {}): DiagnosisInput => ({
  now: 10_000_000, status: status(), clockOffsetMs: 20, periodSecs: 15, audio: null, lastDecodeAt: 9_990_000, sessionStartAt: 9_000_000, ...over,
});
const ids = (i: DiagnosisInput) => diagnose(i).map((x) => `${x.id}:${x.severity}`);

describe('diagnose', () => {
  it('says nothing when everything is fine, or when there is not enough data', () => {
    expect(diagnose(base())).toEqual([]);
    expect(diagnose(base({ status: null, clockOffsetMs: null, sessionStartAt: null, lastDecodeAt: null }))).toEqual([]);
  });

  it('judges the clock in proportion to the period: 4 % warns, 8 % is a problem', () => {
    expect(ids(base({ clockOffsetMs: 590 }))).toEqual([]);
    expect(ids(base({ clockOffsetMs: 600 }))).toEqual(['clock:warning']);
    expect(ids(base({ clockOffsetMs: -1200 }))).toEqual(['clock:problem']);
    // in FT4 il periodo e' la meta': lo stesso scarto pesa il doppio
    expect(ids(base({ clockOffsetMs: 300, periodSecs: 7.5 }))).toEqual(['clock:warning']);
    expect(ids(base({ clockOffsetMs: 300, periodSecs: 15 }))).toEqual([]);
  });

  it('offers the clock settings as the place to fix it', () => {
    expect(diagnose(base({ clockOffsetMs: 1000 }))[0]?.href).toBe('#/impostazioni/clock');
  });

  it('reports an overdriven, a silent and a weak input with different severity', () => {
    const audio = (state: 'overdrive' | 'silent' | 'low' | 'good') => ({ peak: 0.5, rms: 0.1, clipped: 0, state });
    expect(ids(base({ audio: audio('overdrive') }))).toEqual(['audio:problem']);
    expect(ids(base({ audio: audio('silent') }))).toEqual(['audio:warning']);
    expect(ids(base({ audio: audio('low') }))).toEqual(['audio:info']);
    expect(ids(base({ audio: audio('good') }))).toEqual([]);
  });

  it('notices a missing callsign in the engine', () => {
    expect(ids(base({ status: status({ de_call: '  ' }) }))).toEqual(['callsign:warning']);
  });

  it('tells when nothing has been decoded for three minutes while not transmitting', () => {
    expect(ids(base({ lastDecodeAt: 10_000_000 - 179_000 }))).toEqual([]);
    const quiet = diagnose(base({ lastDecodeAt: 10_000_000 - 4 * 60_000 }));
    expect(quiet.map((x) => x.id)).toEqual(['quiet']);
    expect(quiet[0]?.vars.min).toBe(4);
    expect(ids(base({ lastDecodeAt: 10_000_000 - 4 * 60_000, status: status({ transmitting: true }) }))).toEqual([]);
  });

  it('counts the quiet time from the start of the session when nothing was decoded yet', () => {
    expect(ids(base({ lastDecodeAt: null, sessionStartAt: 10_000_000 - 100_000 }))).toEqual([]);
    expect(ids(base({ lastDecodeAt: null, sessionStartAt: 10_000_000 - 200_000 }))).toEqual(['quiet:info']);
  });

  it('puts the most serious first', () => {
    const list = ids(base({ clockOffsetMs: 2000, status: status({ de_call: '' }), lastDecodeAt: 0, audio: { peak: 1, rms: 0.7, clipped: 0.1, state: 'overdrive' } }));
    expect(list[0]).toMatch(/:problem$/);
    expect(list.at(-1)).toBe('quiet:info');
    expect(list.slice(0, 2).every((x) => x.endsWith(':problem'))).toBe(true);
  });
});
