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

const decode = (id: number, tag: DecodedMessage['tag'], text = `msg ${id}`): DecodedMessage => ({
  id, time_ms: id, snr: -10, delta_time: 0, freq_hz: 1500, text, tag,
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

describe('alertKind with the per-kind sound flags', () => {
  it('stays silent for a kind whose sound is switched off', () => {
    expect(alertKind('new_dxcc', { new_dxcc: false })).toBeNull();
    expect(alertKind('new_zone', { new_dxcc: false })).toBe('news');
  });

  it('rings for a kind whose sound was switched on, even a common one', () => {
    expect(alertKind('new_call', { new_call: true })).toBe('news');
    expect(alertKind('worked_call', { worked_call: true })).toBe('news');
    expect(alertKind('cq', { cq: true })).toBe('news');
  });

  it('never rings for the automatic replies', () => {
    expect(alertKind('auto', { auto: true })).toBeNull();
  });

  it('pickAlert honours the flags', () => {
    const list = [decode(9, 'new_dxcc'), decode(8, 'new_zone')];
    expect(pickAlert(list, 7, { new_dxcc: false })).toEqual({ kind: 'news', lastId: 9 });
    expect(pickAlert(list, 7, { new_dxcc: false, new_zone: false })).toEqual({ kind: null, lastId: 9 });
  });
});

describe('a call addressed to me', () => {
  const calling = (id: number) => decode(id, null, 'AA1AA BB2BB -10');

  it('rings its own insistent sound, stronger than the news', () => {
    const list = [calling(9), decode(8, 'new_dxcc')];
    expect(pickAlert(list, 7, undefined, 'AA1AA')).toEqual({ kind: 'me', lastId: 9 });
  });

  it('can be silenced with the my_call sound flag', () => {
    expect(pickAlert([calling(9)], 7, { my_call: false }, 'AA1AA')).toEqual({ kind: null, lastId: 9 });
  });

  it('does not ring for my own messages or without my callsign', () => {
    expect(pickAlert([decode(9, null, 'BB2BB AA1AA -10')], 7, undefined, 'AA1AA').kind).toBeNull();
    expect(pickAlert([calling(9)], 7).kind).toBeNull();
  });

  it('plays four alternating notes', () => {
    const tones: number[] = [];
    playAlert('me', 1, { now: () => 0, tone: (hz) => tones.push(hz) });
    expect(tones).toHaveLength(4);
    expect(tones[0]).toBe(tones[2]);
    expect(tones[1]).toBe(tones[3]);
    expect(tones[0]).not.toBe(tones[1]);
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
