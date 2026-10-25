import { describe, expect, it } from 'vitest';
import type { CatCandidate, CatModel } from '../api/client';
import { DEFAULT_CAT, applyCandidate, catIssue, filterModels, isSerialPort, modelForCandidate, sameCat, visibleFields } from './cat';

const models: CatModel[] = [
  { id: 1, manufacturer: 'Hamlib', model: 'Dummy', status: 'Stable' },
  { id: 1029, manufacturer: 'Yaesu', model: 'FT-2000', status: 'Stable' },
  { id: 1040, manufacturer: 'Yaesu', model: 'FT-2000D', status: 'Stable' },
  { id: 1035, manufacturer: 'Yaesu', model: 'FT-991', status: 'Stable' },
  { id: 3073, manufacturer: 'Icom', model: 'IC-7300', status: 'Stable' },
];

const candidate: CatCandidate = {
  device_key: 'k',
  is_radio: true,
  protocol: 'newcat',
  rig_label: 'Yaesu FT-2000',
  rig_token: 'FT-2000',
  rig_model: null,
  alternatives: [],
  cat_port: 'COM5',
  other_ports: [],
  baud: 38400,
  civ_address: null,
  audio_input: null,
  audio_output: null,
  confidence: 90,
  evidence: 'nome',
};

describe('serial ports and validation', () => {
  it('tells a COM port from a network address', () => {
    expect(isSerialPort('COM3') && isSerialPort('com12') && isSerialPort('/dev/ttyUSB0')).toBe(true);
    expect(isSerialPort('localhost:4532') || isSerialPort('')).toBe(false);
  });

  it('accepts the base configuration and refuses what the core refuses', () => {
    expect(catIssue(DEFAULT_CAT)).toBeNull();
    expect(catIssue({ ...DEFAULT_CAT, port: 'COM3 & calc' })).toBe('bad_port');
    expect(catIssue({ ...DEFAULT_CAT, baud: 12345 })).toBe('bad_baud');
    expect(catIssue({ ...DEFAULT_CAT, poll_secs: 0 })).toBe('bad_poll');
    expect(catIssue({ ...DEFAULT_CAT, poll_secs: 100 })).toBe('bad_poll');
    expect(catIssue({ ...DEFAULT_CAT, data_bits: 9 })).toBe('bad_serial');
    expect(catIssue({ ...DEFAULT_CAT, ptt: 'rts' })).toBe('bad_ptt');
    expect(catIssue({ ...DEFAULT_CAT, ptt: 'rts', port: 'COM3' })).toBeNull();
    expect(catIssue({ ...DEFAULT_CAT, rig_name: 'x'.repeat(81) })).toBe('bad_rig_name');
  });
});

describe('the fields shown', () => {
  it('follow backend, port type and PTT method', () => {
    const f = visibleFields({ ...DEFAULT_CAT, port: 'COM3', ptt: 'dtr' }, undefined);
    expect([f.rig, f.baud, f.serial, f.pttPort, f.civ]).toEqual([true, true, true, true, false]);
    const net = visibleFields({ ...DEFAULT_CAT, port: '192.168.1.5:4532' }, undefined);
    expect([net.baud, net.serial]).toEqual([false, false]);
    const omni = visibleFields({ ...DEFAULT_CAT, backend: 'omnirig' }, undefined);
    expect([omni.rig, omni.port, omni.serial, omni.ptt, omni.poll, omni.split]).toEqual([false, false, false, false, false, true]);
    expect(visibleFields({ ...DEFAULT_CAT, port: 'COM3' }, models[4]).civ).toBe(true);
  });
});

describe('model search', () => {
  it('finds by words in any order, ignoring dashes and case', () => {
    expect(filterModels(models, 'yaesu 2000').map((m) => m.id)).toEqual([1029, 1040]);
    expect(filterModels(models, 'ft2000').map((m) => m.id)).toEqual([1029, 1040]);
    expect(filterModels(models, 'IC 7300').map((m) => m.id)).toEqual([3073]);
    expect(filterModels(models, '').length).toBe(5);
    expect(filterModels(models, 'zzz')).toEqual([]);
  });

  it('resolves a detected radio to the exact model first', () => {
    expect(modelForCandidate(models, candidate)?.id).toBe(1029);
    expect(modelForCandidate(models, { ...candidate, rig_token: 'FT-991' })?.id).toBe(1035);
    expect(modelForCandidate(models, { ...candidate, rig_model: 3073 })?.id).toBe(3073);
    expect(modelForCandidate(models, { ...candidate, rig_token: '' })).toBeUndefined();
  });

  it('brings a detected radio into the draft without touching the rest', () => {
    const base = { ...DEFAULT_CAT, split: 'rig' as const, poll_secs: 5 };
    const next = applyCandidate(base, candidate, models[1]);
    expect(next).toMatchObject({ model: 1029, rig_name: 'Yaesu FT-2000', port: 'COM5', baud: 38400, split: 'rig', poll_secs: 5 });
    expect(sameCat(base, next)).toBe(false);
    expect(sameCat(base, { ...base })).toBe(true);
  });
});
