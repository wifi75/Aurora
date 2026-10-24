import { describe, expect, it } from 'vitest';
import { formatWhen, historyParams, periodStart } from './history';

const NOW = 1_800_000_000_000;
const filter = { q: '', band: '', mode: '', period: 'all' as const };

describe('periodStart', () => {
  it('goes back by the chosen period, or not at all', () => {
    expect(periodStart('1h', NOW)).toBe(NOW - 3_600_000);
    expect(periodStart('24h', NOW)).toBe(NOW - 86_400_000);
    expect(periodStart('7d', NOW)).toBe(NOW - 7 * 86_400_000);
    expect(periodStart('all', NOW)).toBeNull();
  });
});

describe('historyParams', () => {
  it('sends only what was chosen', () => {
    expect(historyParams(filter, 0, 100, NOW).toString()).toBe('limit=100');
  });

  it('trims the search and adds band, mode, period and the page', () => {
    const p = historyParams({ q: '  k1abc ', band: '20m', mode: 'FT8', period: '24h' }, 200, 100, NOW);
    expect(Object.fromEntries(p)).toEqual({ q: 'k1abc', band: '20m', mode: 'FT8', from_ms: String(NOW - 86_400_000), limit: '100', offset: '200' });
  });

  it('ignores a search made only of spaces', () => {
    expect(historyParams({ ...filter, q: '   ' }, 0, 50, NOW).has('q')).toBe(false);
  });
});

describe('formatWhen', () => {
  it('writes day/month and the local time', () => {
    expect(formatWhen(new Date(2026, 9, 24, 16, 5, 9).getTime())).toBe('24/10 16:05:09');
  });
});
