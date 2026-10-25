import { describe, expect, it } from 'vitest';
import { LANGS } from '../i18n/core';
import { SOUNDS, SOUND_CATEGORIES, envelope, getSound, playSound, polyphony, previewSound, scheduleSound, soundDuration, stopAll, toneGain } from './soundBank';

/** Un AudioContext finto: registra i nodi creati e le note programmate, senza audio vero. */
function fakeContext(state: 'running' | 'suspended' = 'running') {
  const log = { oscillators: 0, started: [] as number[], resumed: 0 };
  const param = () => ({ value: 0, setValueAtTime() {}, linearRampToValueAtTime() {}, exponentialRampToValueAtTime() {} });
  const node = (): Record<string, unknown> => ({ connect: (n: unknown) => n, disconnect() {} });
  const ctx = {
    currentTime: 1,
    state,
    destination: node(),
    resume() {
      log.resumed++;
      (ctx as { state: string }).state = 'running';
      return Promise.resolve();
    },
    createOscillator() {
      log.oscillators++;
      return { ...node(), type: 'sine', frequency: param(), start: (at: number) => log.started.push(at), stop() {}, onended: null };
    },
    createGain: () => ({ ...node(), gain: param() }),
    createBiquadFilter: () => ({ ...node(), type: 'lowpass', frequency: param(), Q: param() }),
  };
  return { ctx: ctx as unknown as AudioContext, log };
}

describe('the sound bank', () => {
  it('has at least 34 sounds with unique ids', () => {
    expect(SOUNDS.length).toBeGreaterThanOrEqual(34);
    expect(new Set(SOUNDS.map((s) => s.id)).size).toBe(SOUNDS.length);
  });

  it('keeps the four old sounds and the four standard alerts of before', () => {
    for (const id of ['chime', 'bell', 'pip', 'horn', 'rise3', 'single', 'trill4', 'rise4']) expect(getSound(id), id).toBeDefined();
  });

  it('gives every sound a name in the six languages and a known category', () => {
    const categories = SOUND_CATEGORIES.map((c) => c.id);
    for (const s of SOUNDS) {
      expect(s.name).toHaveLength(LANGS.length);
      for (const n of s.name) expect(n.trim(), s.id).not.toBe('');
      expect(categories, s.id).toContain(s.category);
    }
    for (const c of SOUND_CATEGORIES) expect(SOUNDS.some((s) => s.category === c.id), c.id).toBe(true);
  });

  it('keeps every sound short (0.2 to 1.5 s) with valid notes', () => {
    for (const s of SOUNDS) {
      const d = soundDuration(s);
      expect(d, s.id).toBeGreaterThanOrEqual(0.2);
      expect(d, s.id).toBeLessThanOrEqual(1.5);
      expect(s.tones.length, s.id).toBeGreaterThan(0);
      for (const t of s.tones) {
        expect(t.hz, s.id).toBeGreaterThanOrEqual(40);
        expect(t.hz, s.id).toBeLessThanOrEqual(8000);
        if (t.to !== undefined) expect(t.to, s.id).toBeGreaterThan(0);
        expect(t.at, s.id).toBeGreaterThanOrEqual(0);
        expect(t.dur, s.id).toBeGreaterThanOrEqual(0.03);
        expect(t.gain ?? 1, s.id).toBeGreaterThan(0);
        expect(t.gain ?? 1, s.id).toBeLessThanOrEqual(1);
      }
    }
  });

  it('makes every sound different from the others', () => {
    const shapes = new Set(SOUNDS.map((s) => JSON.stringify([s.wave, s.filter, s.tones])));
    expect(shapes.size).toBe(SOUNDS.length);
  });

  it('ramps every note from zero and back to zero, so there are no clicks', () => {
    for (const s of SOUNDS) {
      for (const t of s.tones) {
        const points = envelope(t);
        expect(points[0], s.id).toEqual({ t: 0, g: 0 });
        expect(points.at(-1)!.g, s.id).toBe(0);
        expect(points.at(-1)!.t, s.id).toBeCloseTo(t.dur, 6);
        expect(points.every((p, i) => i === 0 || p.t >= points[i - 1]!.t), s.id).toBe(true);
        // l'attacco non e' istantaneo
        expect(points[1]!.t, s.id).toBeGreaterThanOrEqual(Math.min(0.004, t.dur * 0.2));
      }
    }
  });

  it('keeps the loudness in check: stacked notes are scaled down and the volume is a ceiling', () => {
    for (const s of SOUNDS) {
      for (const t of s.tones) {
        expect(toneGain(s, t, 1), s.id).toBeLessThanOrEqual(0.5);
        expect(toneGain(s, t, 0), s.id).toBe(0);
      }
    }
    expect(polyphony(getSound('chord')!)).toBe(3);
    expect(polyphony(getSound('beep3')!)).toBe(1);
    const chord = getSound('chord')!;
    expect(toneGain(chord, chord.tones[0]!, 1)).toBeLessThan(toneGain(getSound('beeplong')!, getSound('beeplong')!.tones[0]!, 1));
  });

  it('has the Morse of «DX» and «73» with dots and dashes', () => {
    const dx = getSound('cwdx')!.tones;
    expect(dx).toHaveLength(7);
    expect(dx[0]!.dur).toBeCloseTo(dx[1]!.dur * 3, 6);
    expect(getSound('cw73')!.tones).toHaveLength(10);
  });
});

describe('playSound', () => {
  it('schedules every note of the sound on the shared context', () => {
    const { ctx, log } = fakeContext();
    expect(playSound('chime', 0.5, ctx)).toBe(true);
    expect(log.oscillators).toBe(3);
    expect(log.started[1]! - log.started[0]!).toBeCloseTo(0.16, 5);
    const gong = fakeContext();
    expect(scheduleSound(gong.ctx, getSound('gong')!, 1)).toBe(getSound('gong')!.tones.length);
  });

  it('stays silent for an unknown sound, for zero volume and while the context is still suspended', () => {
    const a = fakeContext();
    expect(playSound('non-esiste', 1, a.ctx)).toBe(false);
    expect(playSound('chime', 0, a.ctx)).toBe(false);
    expect(a.log.oscillators).toBe(0);
    const b = fakeContext('suspended');
    expect(playSound('chime', 1, b.ctx)).toBe(false);
    expect(b.log.resumed).toBe(1);
    expect(b.log.oscillators).toBe(0);
    expect(playSound('chime', 1, null)).toBe(false);
  });

  it('a test from a click resumes a suspended context and then plays', async () => {
    const { ctx, log } = fakeContext('suspended');
    previewSound('pip', 1, ctx);
    await Promise.resolve();
    await Promise.resolve();
    expect(log.resumed).toBe(1);
    expect(log.oscillators).toBe(2);
  });

  it('stopAll does not fail with nothing playing or after playing', () => {
    const { ctx } = fakeContext();
    playSound('gong', 1, ctx);
    expect(() => stopAll()).not.toThrow();
    expect(() => stopAll()).not.toThrow();
  });
});
