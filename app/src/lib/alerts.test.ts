import { describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { alertKind, pickAlert, playAlert, playNamed, readAlerts, saveAlerts, type ToneContext } from './alerts';

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

  it('with «sound at the first decoded message» a new batch rings one note, but only when nothing better rang and never for the first look', () => {
    const plain = [decode(9, null, 'CQ CC3CC FN42'), decode(8, null, 'CQ DD4DD JO31')];
    expect(pickAlert(plain, 7, undefined, null)).toEqual({ kind: null, lastId: 9 });
    expect(pickAlert(plain, 7, undefined, null, true)).toEqual({ kind: 'news', lastId: 9 });
    // niente di nuovo dall'ultima volta, e le righe Tx (identificativo negativo) non contano
    expect(pickAlert(plain, 9, undefined, null, true)).toEqual({ kind: null, lastId: 9 });
    expect(pickAlert([decode(-1, null, 'CQ AA1AA JN45')], 0, undefined, null, true).kind).toBeNull();
    expect(pickAlert(plain, null, undefined, null, true)).toEqual({ kind: null, lastId: 9 });
    // un paese nuovo suona come prima, non una nota sola in piu'
    expect(pickAlert([decode(9, 'new_dxcc')], 7, undefined, null, true).kind).toBe('dxcc');
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

  it('the JTDX alerts: call to me, the DX call, and each «new» type with its «per band» variant', () => {
    const flags = { alertMyCall: false, alertDxCall: false, alertCqz: false, alertItu: false, alertDxcc: false, alertGrid: false, alertPx: false, alertCqzOb: false, alertItuOb: false, alertDxccOb: false, alertGridOb: false, alertPxOb: false };
    const run = (list: DecodedMessage[], f: Partial<typeof flags>, dx?: string) => pickAlert(list, 0, undefined, 'AA1AA', false, { flags: { ...flags, ...f }, dxCall: dx }).alert;
    expect(run([decode(1, 'new_dxcc')], {})).toBe(false);
    expect(run([decode(1, 'new_dxcc')], { alertDxcc: true })).toBe(true);
    // «per banda»: il nuovo su banda ha il suo flag, separato
    expect(run([decode(1, 'new_dxcc_band')], { alertDxcc: true })).toBe(false);
    expect(run([decode(1, 'new_dxcc_band')], { alertDxccOb: true })).toBe(true);
    expect(run([decode(1, 'new_zone')], { alertCqz: true })).toBe(true);
    expect(run([decode(1, 'new_itu_band')], { alertItuOb: true })).toBe(true);
    expect(run([decode(1, 'new_grid')], { alertGrid: true })).toBe(true);
    expect(run([decode(1, 'new_px_band')], { alertPxOb: true })).toBe(true);
    expect(run([decode(1, 'new_call')], { alertDxcc: true, alertGrid: true })).toBe(false);
    expect(run([decode(1, null, 'AA1AA BB2BB -10')], { alertMyCall: true })).toBe(true);
    expect(run([decode(1, null, 'AA1AA BB2BB -10')], {})).toBe(false);
    expect(run([decode(1, null, 'CQ BB2BB FN42')], { alertDxCall: true }, 'bb2bb')).toBe(true);
    expect(run([decode(1, null, 'CQ BB2BBX FN42')], { alertDxCall: true }, 'BB2BB')).toBe(false);
    // le righe Tx (identificativo negativo) e i messaggi gia' visti non suonano
    expect(run([decode(-1, 'new_dxcc')], { alertDxcc: true })).toBe(false);
    expect(pickAlert([decode(5, 'new_dxcc')], 5, undefined, null, false, { flags: { ...flags, alertDxcc: true } }).alert).toBe(false);
  });

  it('plays the chosen sound, each one different, and nothing at zero volume', () => {
    const lengths = new Set<string>();
    for (const name of ['chime', 'bell', 'pip', 'horn'] as const) {
      const a = fakeContext();
      playNamed(name, 0.5, a.ctx);
      expect(a.tones.length).toBeGreaterThan(1);
      lengths.add(JSON.stringify(a.tones.map((t) => t.hz)));
    }
    expect(lengths.size).toBe(4);
    const z = fakeContext();
    playNamed('bell', 0, z.ctx);
    expect(z.tones).toHaveLength(0);
  });

  it('remembers the sound and the silence while transmitting', () => {
    const store = memory();
    saveAlerts(store, { enabled: true, volume: 0.3, sound: 'horn', muteWhileTx: false });
    expect(readAlerts(store)).toEqual({ enabled: true, volume: 0.3, sound: 'horn', muteWhileTx: false });
    expect(readAlerts(memory({ 'aurora-alerts': '{"enabled":true,"sound":"boh"}' })).sound).toBe('chime');
    expect(readAlerts(memory()).muteWhileTx).toBe(true);
  });
});
