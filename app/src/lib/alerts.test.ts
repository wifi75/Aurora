import { describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { alertKind, pickAlert, playAlert, readAlerts, saveAlerts, type ToneContext } from './alerts';

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

const decode = (id: number, tag: DecodedMessage['tag']): DecodedMessage => ({
  id, time_ms: id, snr: -10, delta_time: 0, freq_hz: 1500, text: `msg ${id}`, tag,
});

describe('alertKind', () => {
  it('rings loudly for a new country and softly for the other news', () => {
    expect(alertKind('new_dxcc')).toBe('dxcc');
    expect(alertKind('new_dxcc_band')).toBe('news');
    expect(alertKind('new_zone')).toBe('news');
    expect(alertKind('new_grid')).toBe('news');
  });

  it('stays silent for plain CQ, automatic messages and untagged ones', () => {
    expect(alertKind('cq')).toBeNull();
    expect(alertKind('auto')).toBeNull();
    expect(alertKind(null)).toBeNull();
  });
});

describe('preferences', () => {
  it('start silent at half volume', () => {
    expect(readAlerts(memory())).toEqual({ enabled: false, volume: 0.5 });
    expect(readAlerts(null)).toEqual({ enabled: false, volume: 0.5 });
  });

  it('remember what was saved', () => {
    const store = memory();
    saveAlerts(store, { enabled: true, volume: 0.8 });
    expect(readAlerts(store)).toEqual({ enabled: true, volume: 0.8 });
  });

  it('ignore garbage and clamp the volume', () => {
    expect(readAlerts(memory({ 'aurora-alerts': 'non json' }))).toEqual({ enabled: false, volume: 0.5 });
    expect(readAlerts(memory({ 'aurora-alerts': '{"enabled":true,"volume":7}' }))).toEqual({ enabled: true, volume: 1 });
    expect(readAlerts(memory({ 'aurora-alerts': '{"enabled":"si","volume":"alto"}' }))).toEqual({ enabled: false, volume: 0.5 });
  });

  it('survive a missing or broken storage', () => {
    expect(readAlerts(broken)).toEqual({ enabled: false, volume: 0.5 });
    expect(() => saveAlerts(broken, { enabled: true, volume: 1 })).not.toThrow();
    expect(() => saveAlerts(null, { enabled: true, volume: 1 })).not.toThrow();
  });
});

describe('pickAlert', () => {
  it('looks only at messages newer than the last seen and picks the strongest', () => {
    const list = [decode(9, 'new_grid'), decode(8, 'new_dxcc'), decode(7, 'new_dxcc')];
    expect(pickAlert(list, 7)).toEqual({ kind: 'dxcc', lastId: 9 });
    expect(pickAlert(list, 8)).toEqual({ kind: 'news', lastId: 9 });
  });

  it('rings nothing when nothing new arrived', () => {
    const list = [decode(3, 'new_dxcc')];
    expect(pickAlert(list, 3)).toEqual({ kind: null, lastId: 3 });
    expect(pickAlert([], 5)).toEqual({ kind: null, lastId: 5 });
  });

  it('starts from the current messages without ringing for the backlog', () => {
    expect(pickAlert([decode(3, 'new_dxcc')], null)).toEqual({ kind: null, lastId: 3 });
  });
});

describe('playAlert', () => {
  function fakeContext() {
    const tones: Array<{ hz: number; gain: number; at: number }> = [];
    const ctx: ToneContext = {
      now: () => 10,
      tone: (hz, gain, at) => {
        tones.push({ hz, gain, at });
      },
    };
    return { ctx, tones };
  }

  it('plays three rising notes for a new country, scaled by the volume', () => {
    const { ctx, tones } = fakeContext();
    playAlert('dxcc', 0.5, ctx);
    expect(tones).toHaveLength(3);
    expect(tones[0]!.hz).toBeLessThan(tones[2]!.hz);
    expect(tones.every((t) => t.gain > 0 && t.gain <= 0.5)).toBe(true);
    expect(tones[1]!.at).toBeGreaterThan(tones[0]!.at);
  });

  it('plays a single note for other news and nothing at zero volume', () => {
    const a = fakeContext();
    playAlert('news', 0.5, a.ctx);
    expect(a.tones).toHaveLength(1);
    const b = fakeContext();
    playAlert('dxcc', 0, b.ctx);
    expect(b.tones).toHaveLength(0);
  });
});
