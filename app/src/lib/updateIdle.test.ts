import { describe, expect, it } from 'vitest';
import type { StationStatus } from '../api/schemas';
import { IDLE_AFTER_MS, getUpdateWhenIdle, isStationIdle, setUpdateWhenIdle } from './updateIdle';

const base = { dial_hz: 14074000, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: true, de_call: 'IU3ABC', de_grid: 'IO91wm' } as StationStatus;

describe('isStationIdle', () => {
  it('e ferma solo dopo dieci minuti senza toccare nulla', () => {
    expect(isStationIdle(base, IDLE_AFTER_MS - 1)).toBe(false);
    expect(isStationIdle(base, IDLE_AFTER_MS)).toBe(true);
  });

  it('non e ferma mentre trasmette o con la trasmissione abilitata', () => {
    expect(isStationIdle({ ...base, transmitting: true }, IDLE_AFTER_MS)).toBe(false);
    expect(isStationIdle({ ...base, tx_enabled: true }, IDLE_AFTER_MS)).toBe(false);
  });

  it('non e ferma con un QSO aperto o una stazione in coda', () => {
    expect(isStationIdle({ ...base, dx_call: 'IU3XYZ' }, IDLE_AFTER_MS)).toBe(false);
    expect(isStationIdle({ ...base, queue: ['IU3XYZ'] }, IDLE_AFTER_MS)).toBe(false);
  });

  it('senza lo stato della stazione non e ferma', () => {
    expect(isStationIdle(null, IDLE_AFTER_MS)).toBe(false);
  });
});

describe('preferenza', () => {
  it('di base e spenta e si ricorda', () => {
    window.localStorage.clear();
    expect(getUpdateWhenIdle()).toBe(false);
    setUpdateWhenIdle(true);
    expect(getUpdateWhenIdle()).toBe(true);
    setUpdateWhenIdle(false);
    expect(getUpdateWhenIdle()).toBe(false);
  });
});
