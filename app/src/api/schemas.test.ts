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
        decoding: true, de_call: 'IU3CYV', de_grid: 'JN55',
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
