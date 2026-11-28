import { describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { defaultSpecialCq } from './specialCqPrefs';
import { pickSpecialCq, SPECIAL_COOLDOWN_MS } from './specialCqSound';

const msg = (id: number, text: string): DecodedMessage => ({ id, time_ms: 0, snr: -10, delta_time: 0.1, freq_hz: 1200, text, tag: null });
const on = () => {
  const p = defaultSpecialCq();
  return { ...p, enabled: true, sound: { ...p.sound, POTA: true } };
};

describe('pickSpecialCq', () => {
  it('alla prima chiamata ricorda solo dove siamo', () => {
    expect(pickSpecialCq([msg(5, 'CQ POTA IU3ABC IO91')], null, on(), new Map(), 0)).toEqual({ lastId: 5 });
  });

  it('suona al primo CQ POTA di un nominativo, poi tace per 10 minuti (anche con /P)', () => {
    const seen = new Map<string, number>();
    const first = pickSpecialCq([msg(1, 'CQ POTA IU3ABC IO91')], 0, on(), seen, 1000);
    expect(first).toEqual({ lastId: 1, call: 'IU3ABC', tag: 'POTA' });
    expect(pickSpecialCq([msg(2, 'CQ POTA IU3ABC/P IO91')], 1, on(), seen, 1000 + 60_000).call).toBeUndefined();
    expect(pickSpecialCq([msg(3, 'CQ POTA IU3ABC IO91')], 2, on(), seen, 1000 + SPECIAL_COOLDOWN_MS).call).toBe('IU3ABC');
  });

  it('tace se il tag non ha l\'avviso, se l\'interruttore e\' spento o per i CQ normali', () => {
    const seen = new Map<string, number>();
    expect(pickSpecialCq([msg(1, 'CQ SOTA IU3ABC IO91')], 0, on(), seen, 0).call).toBeUndefined();
    expect(pickSpecialCq([msg(2, 'CQ POTA IU3ABD IO91')], 1, { ...on(), enabled: false }, seen, 0)).toEqual({ lastId: 2 });
    expect(pickSpecialCq([msg(3, 'CQ DX IU3ABE IO91'), msg(4, 'CQ IU3ABF IO91')], 2, on(), seen, 0)).toEqual({ lastId: 4 });
    expect(pickSpecialCq([msg(5, 'CQ POTA IU3ABC IO91')], null, defaultSpecialCq(), seen, 0)).toEqual({ lastId: 5 });
  });

  it('i messaggi gia\' visti e le righe Tx non suonano', () => {
    const seen = new Map<string, number>();
    expect(pickSpecialCq([msg(1, 'CQ POTA IU3ABC IO91'), msg(-1, 'CQ POTA IU3ABD IO91')], 1, on(), seen, 0).call).toBeUndefined();
  });
});
