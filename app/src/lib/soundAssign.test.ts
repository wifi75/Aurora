import { beforeEach, describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { SOUNDS, hasSound } from './soundBank';
import { AUTO_SOUNDS, SOUND_EVENTS, emptyAssign, resetNotified, eventOfAlert, gateOn, migrateLegacy, pickSoundEvent, readAssign, saveAssign, soundFor, type Assign, type RingState } from './soundAssign';

function memory(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return { data, getItem: (k: string) => data[k] ?? null, setItem: (k: string, v: string) => { data[k] = v; } };
}
const broken = { getItem: () => { throw new Error('no'); }, setItem: () => { throw new Error('no'); } };

const decode = (id: number, tag: DecodedMessage['tag'], text = `msg ${id}`): DecodedMessage => ({ id, time_ms: id, snr: -10, delta_time: 0, freq_hz: 1500, text, tag });

const NO_ALERTS = { alertMyCall: false, alertDxCall: false, alertCqz: false, alertItu: false, alertDxcc: false, alertGrid: false, alertPx: false, alertCqzOb: false, alertItuOb: false, alertDxccOb: false, alertGridOb: false, alertPxOb: false, beepFirst: false };

function state(assign: Partial<Assign>, over: Partial<RingState> = {}): RingState {
  return { assign: { ...emptyAssign(), ...assign }, sound: {}, view: NO_ALERTS, enabled: true, muteWhileTx: true, transmitting: false, volume: 0.5, ...over };
}

beforeEach(() => resetNotified());

describe('the assignment', () => {
  it('has a different automatic sound for every function, all in the bank', () => {
    const ids = SOUND_EVENTS.map((e) => AUTO_SOUNDS[e]);
    expect(new Set(ids).size).toBe(SOUND_EVENTS.length);
    for (const id of ids) expect(hasSound(id), id).toBe(true);
    expect(SOUNDS.length).toBeGreaterThanOrEqual(SOUND_EVENTS.length);
  });

  it('remembers what is saved, drops unknown sounds and survives a broken storage', () => {
    const store = memory();
    saveAssign(store, { ...emptyAssign(), cq: 'gong', new_dxcc: 'fanfare' });
    expect(readAssign(store)).toMatchObject({ cq: 'gong', new_dxcc: 'fanfare', new_zone: null });
    expect(readAssign(memory({ 'aurora-sound-assign': '{"v":1,"map":{"cq":"boh","wanted":"gong"}}' }))).toMatchObject({ cq: null, wanted: 'gong' });
    expect(readAssign(memory({ 'aurora-sound-assign': 'non json' }))).toEqual(emptyAssign());
    expect(readAssign(broken)).toEqual(emptyAssign());
    expect(() => saveAssign(broken, emptyAssign())).not.toThrow();
  });

  it('maps every alert of «Play alert sound» to a function', () => {
    expect(eventOfAlert('alertMyCall')).toBe('alert_my_call');
    expect(eventOfAlert('alertDxCall')).toBe('alert_dx_call');
    expect(eventOfAlert('alertCqzOb')).toBe('new_zone_band');
    expect(eventOfAlert('alertPx')).toBe('new_px');
  });
});

describe('migration of the old preferences', () => {
  it('assigns nothing when nothing was ever saved', () => {
    expect(migrateLegacy(memory())).toEqual(emptyAssign());
    expect(migrateLegacy(null)).toEqual(emptyAssign());
    expect(readAssign(memory())).toEqual(emptyAssign());
  });

  it('gives the old standard sound to every type that had its sound on', () => {
    const store = memory({ 'aurora-alerts': '{"enabled":true,"volume":0.5,"sound":"bell"}' });
    const a = migrateLegacy(store);
    // di base suonavano: il mio nominativo, il paese nuovo, le novita' (una nota)
    expect(a.my_call).toBe('trill4');
    expect(a.new_dxcc).toBe('rise3');
    expect(a.new_zone).toBe('single');
    expect(a.new_dxcc_band).toBe('single');
    // e l'avviso «Play alert sound» sulla chiamata verso di me, con il suono scelto
    expect(a.alert_my_call).toBe('bell');
    expect(a.wanted).toBe('rise4');
    // quello che non suonava resta senza suono
    expect(a.cq).toBeNull();
    expect(a.worked_call).toBeNull();
    expect(a.alert_dx_call).toBeNull();
    expect(a.first_message).toBeNull();
  });

  it('follows the saved flags: a sound switched off stays without sound, one switched on gets its sound', () => {
    const store = memory({
      'aurora-notify': JSON.stringify({ sound: { my_call: false, cq: true } }),
      'aurora-notify-view': JSON.stringify({ alertDxCall: true, alertGrid: true, beepFirst: true }),
      'aurora-alerts': '{"enabled":true,"sound":"horn"}',
    });
    const a = migrateLegacy(store);
    expect(a.my_call).toBeNull();
    expect(a.cq).toBe('single');
    expect(a.alert_dx_call).toBe('horn');
    expect(a.new_grid).toBe('horn');
    expect(a.first_message).toBe('single');
  });
});

describe('what rings', () => {
  const sound = { new_dxcc: true, cq: true };

  it('rings the assigned sound only when the alert is on, the function has a sound and the sound alerts are on', () => {
    const st = state({ new_dxcc: 'fanfare', cq: null }, { sound });
    expect(soundFor('new_dxcc', st)).toBe('fanfare');
    // acceso ma senza suono: tace
    expect(soundFor('cq', st)).toBeNull();
    // con suono ma l'avviso spento: tace
    expect(soundFor('new_zone', state({ new_zone: 'gong' }, { sound }))).toBeNull();
    // avvisi sonori spenti: tace
    expect(soundFor('new_dxcc', state({ new_dxcc: 'fanfare' }, { sound, enabled: false }))).toBeNull();
    // un suono che non esiste: tace
    expect(soundFor('new_dxcc', state({ new_dxcc: 'boh' }, { sound }))).toBeNull();
  });

  it('is silent while transmitting, except the Tx message itself', () => {
    const st = state({ new_dxcc: 'fanfare', tx_msg: 'tictoc' }, { sound: { new_dxcc: true, tx_msg: true }, transmitting: true });
    expect(soundFor('new_dxcc', st)).toBeNull();
    expect(soundFor('tx_msg', st)).toBe('tictoc');
    expect(soundFor('new_dxcc', { ...st, muteWhileTx: false })).toBe('fanfare');
  });

  it('the «Play alert sound» flags and the sound flag both switch a type on', () => {
    expect(gateOn('new_zone', {}, { ...NO_ALERTS, alertCqz: true })).toBe(true);
    expect(gateOn('new_zone_band', {}, { ...NO_ALERTS, alertCqz: true })).toBe(false);
    expect(gateOn('new_zone_band', {}, { ...NO_ALERTS, alertCqzOb: true })).toBe(true);
    expect(gateOn('cq', { cq: true }, NO_ALERTS)).toBe(true);
    expect(gateOn('cq', {}, NO_ALERTS)).toBe(false);
    expect(gateOn('wanted', {}, NO_ALERTS)).toBe(true);
  });
});

describe('pickSoundEvent', () => {
  const all = (over: Partial<RingState> = {}) =>
    state({ ...AUTO_SOUNDS }, { sound: { new_dxcc: true, new_zone: true, cq: true, my_call: true }, view: { ...NO_ALERTS, alertMyCall: true, alertDxCall: true }, ...over });

  it('starts from the current messages without ringing for the backlog', () => {
    expect(pickSoundEvent([decode(3, 'new_dxcc')], null, all())).toEqual({ event: null, sound: null, lastId: 3 });
  });

  it('plays the sound assigned to the type of the new message, a different one for each', () => {
    const a = pickSoundEvent([decode(9, 'new_dxcc')], 7, all());
    const b = pickSoundEvent([decode(9, 'new_zone')], 7, all());
    const c = pickSoundEvent([decode(9, 'cq')], 7, all());
    expect([a.event, b.event, c.event]).toEqual(['new_dxcc', 'new_zone', 'cq']);
    expect(new Set([a.sound, b.sound, c.sound]).size).toBe(3);
  });

  it('rings nothing for a type without a sound, even with its alert on', () => {
    const st = all();
    st.assign.new_dxcc = null;
    expect(pickSoundEvent([decode(9, 'new_dxcc')], 7, st)).toEqual({ event: null, sound: null, lastId: 9 });
  });

  it('a call to me beats the news; the next function plays if the first has no sound', () => {
    const list = [decode(9, null, 'AA1AA BB2BB -10'), decode(8, 'new_dxcc')];
    expect(pickSoundEvent(list, 7, all(), 'AA1AA').event).toBe('my_call');
    const st = all();
    st.assign.my_call = null;
    expect(pickSoundEvent(list, 7, st, 'AA1AA').event).toBe('alert_my_call');
    st.assign.alert_my_call = null;
    expect(pickSoundEvent(list, 7, st, 'AA1AA').event).toBe('new_dxcc');
  });

  it('the chosen DX call has its own sound', () => {
    const r = pickSoundEvent([decode(9, null, 'CQ BB2BB FN42')], 7, all(), 'AA1AA', 'bb2bb');
    expect(r.event).toBe('alert_dx_call');
    expect(r.sound).toBe(AUTO_SOUNDS.alert_dx_call);
    expect(pickSoundEvent([decode(9, null, 'CQ BB2BBX FN42')], 7, all(), 'AA1AA', 'BB2BB').event).toBeNull();
  });

  it('the first decoded message rings only when nothing better did, and never for Tx lines or messages already seen', () => {
    const st = all({ view: { ...NO_ALERTS, beepFirst: true } });
    const plain = [decode(9, null, 'CQ CC3CC FN42')];
    expect(pickSoundEvent(plain, 7, st).event).toBe('first_message');
    expect(pickSoundEvent(plain, 9, st).event).toBeNull();
    expect(pickSoundEvent([decode(-1, null, 'CQ AA1AA JN45')], 0, st).event).toBeNull();
    expect(pickSoundEvent([decode(9, 'new_dxcc')], 7, st).event).toBe('new_dxcc');
  });

  it('never rings for the automatic replies', () => {
    expect(pickSoundEvent([decode(9, 'auto')], 7, all()).event).toBeNull();
  });

  it('is silent while transmitting', () => {
    expect(pickSoundEvent([decode(9, 'new_dxcc')], 7, all({ transmitting: true })).event).toBeNull();
  });
});

describe('a station rings only the first time', () => {
  it('the same station retransmitting does not ring again for the same function', () => {
    const st = state({ my_call: 'chime' }, { sound: { my_call: true } });
    const first = pickSoundEvent([decode(1, null, 'AA1AA BB2BB -10')], null, st, 'AA1AA');
    const again = pickSoundEvent([decode(1, null, 'AA1AA BB2BB -10'), decode(2, null, 'AA1AA BB2BB -10')], first.lastId, st, 'AA1AA');
    expect(again.event).toBe('my_call');
    const third = pickSoundEvent([decode(1, null, 'x'), decode(2, null, 'x'), decode(3, null, 'AA1AA BB2BB -10')], again.lastId, st, 'AA1AA');
    expect(third.event).toBeNull();
    const other = pickSoundEvent([decode(4, null, 'AA1AA CC3CC -10')], third.lastId, st, 'AA1AA');
    expect(other.event).toBe('my_call');
  });
});
