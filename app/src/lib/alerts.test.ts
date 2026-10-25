import { describe, expect, it } from 'vitest';
import { readAlerts, saveAlerts } from './alerts';

function memory(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return {
    data,
    getItem: (k: string) => data[k] ?? null,
    setItem: (k: string, v: string) => {
      data[k] = v;
    },
  };
}

const broken = {
  getItem: () => {
    throw new Error('archivio non disponibile');
  },
  setItem: () => {
    throw new Error('archivio non disponibile');
  },
};

describe('preferences', () => {
  it('start silent at half volume', () => {
    expect(readAlerts(memory())).toEqual({ enabled: false, volume: 0.5, sound: 'chime', muteWhileTx: true });
    expect(readAlerts(null)).toEqual({ enabled: false, volume: 0.5, sound: 'chime', muteWhileTx: true });
  });

  it('remember what was saved', () => {
    const store = memory();
    saveAlerts(store, { enabled: true, volume: 0.8, sound: 'chime', muteWhileTx: true });
    expect(readAlerts(store)).toEqual({ enabled: true, volume: 0.8, sound: 'chime', muteWhileTx: true });
  });

  it('ignore garbage and clamp the volume', () => {
    expect(readAlerts(memory({ 'aurora-alerts': 'non json' }))).toEqual({ enabled: false, volume: 0.5, sound: 'chime', muteWhileTx: true });
    expect(readAlerts(memory({ 'aurora-alerts': '{"enabled":true,"volume":7}' }))).toEqual({ enabled: true, volume: 1, sound: 'chime', muteWhileTx: true });
    expect(readAlerts(memory({ 'aurora-alerts': '{"enabled":"si","volume":"alto"}' }))).toEqual({ enabled: false, volume: 0.5, sound: 'chime', muteWhileTx: true });
  });

  it('survive a missing or broken storage', () => {
    expect(readAlerts(broken)).toEqual({ enabled: false, volume: 0.5, sound: 'chime', muteWhileTx: true });
    expect(() => saveAlerts(broken, { enabled: true, volume: 1, sound: 'chime', muteWhileTx: true })).not.toThrow();
    expect(() => saveAlerts(null, { enabled: true, volume: 1, sound: 'chime', muteWhileTx: true })).not.toThrow();
  });
});

describe('the old single alert sound', () => {
  it('remembers the sound and the silence while transmitting', () => {
    const store = memory();
    saveAlerts(store, { enabled: true, volume: 0.3, sound: 'horn', muteWhileTx: false });
    expect(readAlerts(store)).toEqual({ enabled: true, volume: 0.3, sound: 'horn', muteWhileTx: false });
    expect(readAlerts(memory({ 'aurora-alerts': '{"enabled":true,"sound":"boh"}' })).sound).toBe('chime');
    expect(readAlerts(memory()).muteWhileTx).toBe(true);
  });
});
