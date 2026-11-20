import { describe, expect, it } from 'vitest';
import { chainDots } from './txChain';

const states = (c: Parameters<typeof chainDots>[0]) => chainDots(c).map((d) => d.state);

describe('chainDots', () => {
  it('senza trasmissioni non mostra niente', () => {
    expect(chainDots(null)).toEqual([]);
    expect(chainDots(undefined)).toEqual([]);
  });

  it('tutto a posto: modo digitale, audio con picco, PTT letto acceso', () => {
    expect(states({ radio_mode: 'USB', peak_dbfs: -12, ptt_read: true })).toEqual(['ok', 'ok', 'ok']);
  });

  it('segnala un modo radio che non va, l audio muto e il PTT che la radio dice spento', () => {
    expect(states({ radio_mode: 'CW', audio_muted: true, peak_dbfs: null, ptt_read: false })).toEqual(['bad', 'bad', 'bad']);
  });

  it('quello che non si puo leggere resta grigio, senza errore', () => {
    expect(states({ radio_mode: null, peak_dbfs: -6, ptt_read: null })).toEqual(['unknown', 'ok', 'unknown']);
  });

  it('il suggerimento porta il modo letto e il picco', () => {
    const dots = chainDots({ radio_mode: 'PKTUSB', peak_dbfs: -3.04, ptt_read: true });
    expect(dots[0]?.value).toBe('PKTUSB');
    expect(dots[1]?.value).toBe('-3.0 dBFS');
  });
});
