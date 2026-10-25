import { describe, expect, it } from 'vitest';
import { EventSchema, HealthSchema } from './schemas';

describe('EventSchema', () => {
  it('accepts a decode event', () => {
    const parsed = EventSchema.parse({
      type: 'decode',
      id: 7,
      time_ms: 45015000,
      snr: -12,
      delta_time: 0.3,
      freq_hz: 1500,
      text: 'CQ DX JA1XYZ PM95',
      tag: 'new_dxcc',
    });
    expect(parsed.type).toBe('decode');
    if (parsed.type === 'decode') expect(parsed.tag).toBe('new_dxcc');
  });

  it('accepts a status with the CAT meters, with only some of them, or without', () => {
    const base = { type: 'status', dial_hz: 14074000, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: true, de_call: 'AA1AA', de_grid: 'FN42' };
    const full = EventSchema.safeParse({ ...base, meters: { s_points: 7, s_over_db: 0, power_w: 5, swr: 1.2, alc: 0.3, comp_db: 4, id_a: 1.1, vdd_v: 13.8 } });
    const some = EventSchema.safeParse({ ...base, meters: { s_points: 3 } });
    const none = EventSchema.safeParse(base);
    const nul = EventSchema.safeParse({ ...base, meters: null });
    expect([full.success, some.success, none.success, nul.success]).toEqual([true, true, true, true]);
    expect(EventSchema.safeParse({ ...base, meters: { swr: 'alto' } }).success).toBe(false);
    if (some.success && some.data.type === 'status') expect(some.data.meters?.power_w).toBeUndefined();
  });

  it('accepts the estimated flag on the S-meter, optional and boolean only', () => {
    const base = { type: 'status', dial_hz: 14074000, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: true, de_call: 'AA1AA', de_grid: 'FN42' };
    const est = EventSchema.safeParse({ ...base, meters: { s_points: 4, s_over_db: 0, estimated: true } });
    expect(est.success).toBe(true);
    if (est.success && est.data.type === 'status') expect(est.data.meters?.estimated).toBe(true);
    const real = EventSchema.safeParse({ ...base, meters: { s_points: 4 } });
    if (real.success && real.data.type === 'status') expect(real.data.meters?.estimated).toBeUndefined();
    expect(EventSchema.safeParse({ ...base, meters: { estimated: 'si' } }).success).toBe(false);
  });

  it('accepts the new tags and the country, and a decode without country', () => {
    const base = { type: 'decode', id: 1, time_ms: 0, snr: -1, delta_time: 0, freq_hz: 100, text: 'X' };
    const a = EventSchema.safeParse({ ...base, tag: 'new_dxcc_band', country: 'Giappone' });
    const b = EventSchema.safeParse({ ...base, tag: 'new_grid' });
    expect(a.success).toBe(true);
    expect(b.success).toBe(true);
    if (a.success && a.data.type === 'decode') expect(a.data.country).toBe('Giappone');
  });

  it('accepts a decode event without tag (null)', () => {
    const parsed = EventSchema.safeParse({
      type: 'decode', id: 1, time_ms: 0, snr: -1, delta_time: 0, freq_hz: 100, text: 'X', tag: null,
    });
    expect(parsed.success).toBe(true);
  });

  it('accepts status and tx_halted events', () => {
    expect(
      EventSchema.safeParse({
        type: 'status', dial_hz: 14074000, mode: 'FT8', tx_enabled: true, transmitting: false,
        decoding: true, de_call: 'AA1AA', de_grid: 'FN42',
      }).success,
    ).toBe(true);
    expect(EventSchema.safeParse({ type: 'tx_halted' }).success).toBe(true);
  });

  it('rejects unknown type', () => {
    expect(EventSchema.safeParse({ type: 'esplosione' }).success).toBe(false);
  });

  it('rejects a decode with a wrong field type', () => {
    expect(
      EventSchema.safeParse({
        type: 'decode', id: 'sette', time_ms: 0, snr: 0, delta_time: 0, freq_hz: 0, text: '', tag: null,
      }).success,
    ).toBe(false);
  });
});

describe('HealthSchema', () => {
  it('requires name, version and author', () => {
    expect(HealthSchema.safeParse({ status: 'ok', name: 'Aurora', version: '0.1.0', author: 'Tiziano Cassone' }).success).toBe(true);
    expect(HealthSchema.safeParse({ status: 'ok', name: 'Aurora' }).success).toBe(false);
  });
});
