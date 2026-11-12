import { describe, expect, it } from 'vitest';
import type { StationStatus } from '../api/schemas';
import { diagnose, parseContestNotice, parseSlowPc, slowerDepth, type DiagnosisInput } from './diagnosis';

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

  it('says nothing about the input while transmitting (many radios mute the USB audio in TX)', () => {
    const audio = (state: 'overdrive' | 'silent' | 'low') => ({ peak: 0, rms: 0, clipped: 0, state });
    const tx = status({ transmitting: true });
    expect(ids(base({ status: tx, audio: audio('silent') }))).toEqual([]);
    expect(ids(base({ status: tx, audio: audio('overdrive') }))).toEqual([]);
    expect(ids(base({ status: tx, audio: audio('low') }))).toEqual([]);
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

describe('notices from the engine', () => {
  it('shows a recent notice with its fix page and forgets an old one', () => {
    const base = { now: 1_000_000, status: null, clockOffsetMs: null, periodSecs: 15, audio: null, lastDecodeAt: null, sessionStartAt: null };
    const recent = diagnose({ ...base, notices: { audio_loss: { detail: '12', at: 990_000 } } });
    expect(recent).toEqual([expect.objectContaining({ id: 'audio_loss', href: '#/impostazioni/audio' })]);
    expect(diagnose({ ...base, now: 990_000 + 11 * 60_000, notices: { audio_loss: { detail: '12', at: 990_000 } } })).toEqual([]);
  });

  it('shows a serial port held by another program as a problem with the Radio page', () => {
    const base = { now: 1_000_000, status: null, clockOffsetMs: null, periodSecs: 15, audio: null, lastDecodeAt: null, sessionStartAt: null };
    const detail = "La porta COM8 e' occupata da OmniRig: chiudi OmniRig oppure scegli OmniRig come sistema di controllo in Impostazioni > Radio";
    expect(diagnose({ ...base, notices: { port_busy: { detail, at: 990_000 } } })).toEqual([
      expect.objectContaining({ id: 'port_busy', severity: 'problem', href: '#/impostazioni/radio', vars: { detail, at: 990_000 } }),
    ]);
  });

  it('drops the busy port notice once the CAT reads connected after it', () => {
    const base = { now: 1_000_000, status: null, clockOffsetMs: null, periodSecs: 15, audio: null, lastDecodeAt: null, sessionStartAt: null };
    const notices = { port_busy: { detail: 'occupata', at: 990_000 } };
    expect(diagnose({ ...base, notices, cat: { connected: true, at: 995_000 } })).toEqual([]);
    // uno stato collegato letto prima dell'avviso, o la radio non collegata, non lo tolgono
    expect(diagnose({ ...base, notices, cat: { connected: true, at: 980_000 } })).toHaveLength(1);
    expect(diagnose({ ...base, notices, cat: { connected: false, at: 995_000 } })).toHaveLength(1);
  });
});

describe('contest notice from the engine', () => {
  const band = "Contest: la banda 20m non e' ammessa dal regolamento del contest. Cambia banda o modo prima di trasmettere.";
  const mode = "Contest: il modo FT4 non e' ammesso dal regolamento del contest. Cambia banda o modo prima di trasmettere.";
  const at = 9_990_000;

  it('reads the reason from the engine text', () => {
    expect(parseContestNotice(band)).toEqual({ kind: 'band', value: '20m' });
    expect(parseContestNotice(mode)).toEqual({ kind: 'mode', value: 'FT4' });
    expect(parseContestNotice('Contest: il tuo nominativo composto non permette i messaggi di contest: ...')).toEqual({ kind: 'myCompound' });
    expect(parseContestNotice('Contest: il nominativo composto del DX non permette i messaggi di contest: ...')).toEqual({ kind: 'dxCompound' });
    expect(parseContestNotice('altro')).toBeNull();
  });

  it('shows a band or mode outside the rules as a problem with the Contest page, until band or mode change', () => {
    expect(diagnose(base({ notices: { contest: { detail: band, at } } }))).toEqual([
      expect.objectContaining({ id: 'contest', severity: 'problem', href: '#/contest', vars: expect.objectContaining({ reason: 'band', value: '20m' }) }),
    ]);
    expect(diagnose(base({ status: status({ dial_hz: 7_074_000 }), notices: { contest: { detail: band, at } } }))).toEqual([]);
    expect(ids(base({ status: status({ mode: 'FT4' }), notices: { contest: { detail: mode, at } } }))).toEqual(['contest:problem']);
    expect(diagnose(base({ notices: { contest: { detail: mode, at } } }))).toEqual([]);
  });

  it('warns about a compound callsign and forgets the notice after ten minutes', () => {
    const detail = 'Contest: il nominativo composto del DX non permette i messaggi di contest: il QSO usa i messaggi normali.';
    expect(ids(base({ notices: { contest: { detail, at } } }))).toEqual(['contest:warning']);
    expect(diagnose(base({ now: at + 11 * 60_000, lastDecodeAt: at + 11 * 60_000, notices: { contest: { detail, at } } }))).toEqual([]);
  });
});

describe('slow PC: one step down in decoding depth, never on its own', () => {
  const base = { now: 1_000_000, status: null, clockOffsetMs: null, periodSecs: 15, audio: null, lastDecodeAt: null, sessionStartAt: null };
  const slow = (detail: string, extra: Partial<DiagnosisInput> = {}) => diagnose({ ...base, notices: { slow_pc: { detail, at: 990_000 } }, ...extra });

  it('reads seconds and depth from the engine notice (older engines send only the seconds)', () => {
    expect(parseSlowPc('11.7|3')).toEqual({ secs: '11.7', depth: '3' });
    expect(parseSlowPc('11.7')).toEqual({ secs: '11.7', depth: null });
    expect(parseSlowPc('11.7|9')).toEqual({ secs: '11.7', depth: null });
    expect([slowerDepth('3'), slowerDepth('2'), slowerDepth('1'), slowerDepth(null)]).toEqual(['2', '1', null, null]);
  });

  it('suggests Normal at Deep and Fast at Normal, with the setting page', () => {
    expect(slow('11.7|3')).toEqual([{ id: 'slow_pc', severity: 'warning', href: '#/impostazioni/filters', vars: { detail: '11.7', at: 990_000, depth: '3', next: '2' } }]);
    expect(slow('11.2|2', { decodeDepth: '2' })[0]?.vars).toMatchObject({ depth: '2', next: '1' });
  });

  it('at Fast or with an older engine keeps the general advice without a depth button', () => {
    expect(slow('11.0|1')[0]).toEqual({ id: 'slow_pc', severity: 'warning', vars: { detail: '11.0', at: 990_000, depth: '1' } });
    expect(slow('11.0')[0]).toEqual({ id: 'slow_pc', severity: 'warning', vars: { detail: '11.0', at: 990_000 } });
  });

  it('stays quiet once the depth has changed or the advice was closed for that depth', () => {
    expect(slow('11.7|3', { decodeDepth: '2' })).toEqual([]);
    expect(slow('11.7|3', { slowPcDismissed: ['3'] })).toEqual([]);
    expect(slow('11.2|2', { slowPcDismissed: ['3'] })).toHaveLength(1);
    expect(diagnose({ ...base, now: 990_000 + 11 * 60_000, notices: { slow_pc: { detail: '11.7|3', at: 990_000 } } })).toEqual([]);
  });
});
