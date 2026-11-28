import { describe, expect, it } from 'vitest';
import type { DecodedMessage } from '../api/schemas';
import { defaultSpecialCq } from './specialCqPrefs';
import { markSpecial, pickSpecialCq, SPECIAL_COOLDOWN_MS, specialYieldsTo } from './specialCqSound';

const msg = (id: number, text: string): DecodedMessage => ({ id, time_ms: 0, snr: -10, delta_time: 0.1, freq_hz: 1200, text, tag: null });
const on = () => {
  const p = defaultSpecialCq();
  return { ...p, enabled: true, sound: { ...p.sound, POTA: true } };
};

describe('pickSpecialCq', () => {
  it('alla prima chiamata ricorda solo dove siamo', () => {
    expect(pickSpecialCq([msg(5, 'CQ POTA IU3ABC IO91')], null, on(), new Map(), 0)).toEqual({ lastId: 5, keys: [] });
  });

  it('suona al primo CQ POTA di un nominativo, poi tace per 10 minuti (anche con /P)', () => {
    const seen = new Map<string, number>();
    const first = pickSpecialCq([msg(1, 'CQ POTA IU3ABC IO91')], 0, on(), seen, 1000);
    expect(first).toEqual({ lastId: 1, call: 'IU3ABC', tag: 'POTA', keys: ['IU3ABC'] });
    markSpecial(seen, first.keys, 1000);
    expect(pickSpecialCq([msg(2, 'CQ POTA IU3ABC/P IO91')], 1, on(), seen, 1000 + 60_000).call).toBeUndefined();
    expect(pickSpecialCq([msg(3, 'CQ POTA IU3ABC IO91')], 2, on(), seen, 1000 + SPECIAL_COOLDOWN_MS).call).toBe('IU3ABC');
  });

  it('se ha ceduto a un avviso piu\' importante (non segnato) suona al CQ dopo', () => {
    const seen = new Map<string, number>();
    const first = pickSpecialCq([msg(1, 'CQ POTA IU3ABC IO91')], 0, on(), seen, 0);
    expect(first.call).toBe('IU3ABC');
    // non segnato: al CQ successivo suona
    expect(pickSpecialCq([msg(2, 'CQ POTA IU3ABC IO91')], 1, on(), seen, 15_000).call).toBe('IU3ABC');
  });

  it('cede solo alle chiamate a me, al DX scelto e al DXCC nuovo', () => {
    for (const e of ['my_call', 'alert_my_call', 'alert_dx_call', 'new_dxcc', 'new_dxcc_band'] as const) expect(specialYieldsTo(e)).toBe(true);
    for (const e of ['cq', 'new_call', 'first_message', 'standard_call', 'worked_call'] as const) expect(specialYieldsTo(e)).toBe(false);
    expect(specialYieldsTo(null)).toBe(false);
  });

  it('tace se il tag non ha l\'avviso, se l\'interruttore e\' spento o per i CQ normali', () => {
    const seen = new Map<string, number>();
    expect(pickSpecialCq([msg(1, 'CQ SOTA IU3ABC IO91')], 0, on(), seen, 0).call).toBeUndefined();
    expect(pickSpecialCq([msg(2, 'CQ POTA IU3ABD IO91')], 1, { ...on(), enabled: false }, seen, 0)).toEqual({ lastId: 2, keys: [] });
    expect(pickSpecialCq([msg(3, 'CQ DX IU3ABE IO91'), msg(4, 'CQ IU3ABF IO91')], 2, on(), seen, 0)).toEqual({ lastId: 4, keys: [] });
  });

  it('i messaggi gia\' visti e le righe Tx non suonano', () => {
    expect(pickSpecialCq([msg(1, 'CQ POTA IU3ABC IO91'), msg(-1, 'CQ POTA IU3ABD IO91')], 1, on(), new Map(), 0).call).toBeUndefined();
  });
});
