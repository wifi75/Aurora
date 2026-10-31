import { describe, expect, it } from 'vitest';
import type { ClockState } from '../api/client';
import { translate, type Key } from '../i18n';
import { CLOCK_STALE_MS, clockAlert } from './ClockPill';

const t = (key: Key, vars?: Record<string, string | number>) => translate('it', key, vars);
const NOW = 1_800_000_000_000;
const config = { enabled: true, servers: ['pool.ntp.org'], interval_secs: 900, warn_ms: 200, auto_correct: false, correct_at_start: true };
const measured = (offset: number | null) => ({ at_ms: NOW, server: 'pool.ntp.org', offset_ms: offset, delay_ms: 20, servers_ok: offset === null ? 0 : 1, servers_tried: 1, error: offset === null ? 'nessuna risposta' : null, corrected: false });

describe('clockAlert', () => {
  it('stays silent while the clock is right and was checked recently', () => {
    const state: ClockState = { config, last: measured(-89), last_good_ms: NOW - 60_000 };
    expect(clockAlert(state, NOW, t)).toBeNull();
  });

  it('warns when the clock is out of tolerance', () => {
    expect(clockAlert({ config, last: measured(450), last_good_ms: NOW - 60_000 }, NOW, t)).toBe('Orologio sbagliato di 450 ms');
  });

  it('warns when the clock has not been checked for more than a day, counting days across restarts', () => {
    expect(clockAlert({ config, last: null, last_good_ms: NOW - CLOCK_STALE_MS - 3_600_000 }, NOW, t)).toBe('Orologio non verificato da 25 ore');
    expect(clockAlert({ config, last: measured(null), last_good_ms: NOW - 3 * 24 * 3_600_000 }, NOW, t)).toBe('Orologio non verificato da 3 giorni');
  });

  it('does not alarm before the first check, but does when the first check failed', () => {
    expect(clockAlert({ config, last: null, last_good_ms: null }, NOW, t)).toBeNull();
    expect(clockAlert({ config, last: measured(null), last_good_ms: null }, NOW, t)).toBe('Orologio mai verificato');
  });
});
