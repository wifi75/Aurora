import { describe, expect, it } from 'vitest';
import type { AudioDevices, CatSnapshot } from '../api/client';
import { evaluateTxCheck } from './txCheck';

const cat = (over: { ptt?: string; state?: string; mode?: string | null; pttOn?: boolean; blocked?: boolean; backend?: string } = {}): CatSnapshot =>
  ({
    status: { rig: { state: over.state ?? 'connected', message: 'ok', mode: over.mode === undefined ? 'USB' : over.mode, ptt: over.pttOn ?? false, ptt_blocked: over.blocked ?? false }, message: '' },
    config: { backend: over.backend ?? 'hamlib', ptt: over.ptt ?? 'cat' },
  }) as unknown as CatSnapshot;
const audio = (over: Partial<AudioDevices> = {}) => ({ output_in_use: 'USB Audio CODEC', output_pending: null, output_error: null, ...over }) as AudioDevices;
const level = { attenuation_db: 10, min_db: 0, max_db: 45 };
const states = (i: ReturnType<typeof evaluateTxCheck>) => i.map((x) => `${x.key}:${x.state}`).join(' ');

describe('evaluateTxCheck', () => {
  it('tutto a posto', () => {
    expect(states(evaluateTxCheck({ cat: cat(), audio: audio(), level }))).toBe('cat:ok mode:ok ptt:ok audio:ok level:ok');
  });

  it('segnala radio non collegata, modo CW, PTT gia acceso, audio assente e livello in fondo', () => {
    const items = evaluateTxCheck({ cat: cat({ state: 'disconnected', mode: 'CW', pttOn: true }), audio: audio({ output_in_use: null }), level: { ...level, attenuation_db: 45 } });
    expect(states(items)).toBe('cat:bad mode:bad ptt:unknown audio:bad level:bad');
  });

  it('PTT acceso con la radio collegata e un problema, col VOX non si legge', () => {
    expect(states(evaluateTxCheck({ cat: cat({ pttOn: true }), audio: audio(), level }))).toContain('ptt:bad');
    expect(states(evaluateTxCheck({ cat: cat({ ptt: 'vox' }), audio: audio(), level }))).toContain('ptt:unknown');
  });

  it('senza CAT la radio non si legge e senza dati il livello e grigio', () => {
    expect(states(evaluateTxCheck({ cat: cat({ backend: 'none', mode: null, state: 'disconnected' }), audio: null, level: null }))).toBe('cat:unknown mode:unknown ptt:unknown audio:bad level:unknown');
  });

  it('un errore del dispositivo audio e un problema con il testo del sistema', () => {
    const [, , , a] = evaluateTxCheck({ cat: cat(), audio: audio({ output_error: 'device busy' }), level });
    expect(a?.state).toBe('bad');
    expect(a?.value).toBe('device busy');
  });
});
