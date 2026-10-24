import { describe, expect, it } from 'vitest';
import { bandOf, formatClock, formatMhz } from './format';

describe('format', () => {
  it('formats the dial frequency in MHz', () => {
    expect(formatMhz(14_074_000)).toBe('14.0740 MHz');
    expect(formatMhz(7_074_000)).toBe('7.0740 MHz');
  });

  it('formats milliseconds since midnight as HH:MM:SS', () => {
    expect(formatClock(45_015_000)).toBe('12:30:15');
    expect(formatClock(0)).toBe('00:00:00');
    expect(formatClock(86_399_000)).toBe('23:59:59');
  });

  it('wraps a value past midnight instead of overflowing', () => {
    expect(formatClock(86_400_000 + 1000)).toBe('00:00:01');
  });

  it('finds the amateur band', () => {
    expect(bandOf(14_074_000)).toBe('20m');
    expect(bandOf(7_074_000)).toBe('40m');
    expect(bandOf(50_313_000)).toBe('6m');
  });

  it('returns null outside the amateur bands', () => {
    expect(bandOf(1_000_000)).toBeNull();
    expect(bandOf(100_000_000)).toBeNull();
  });
});
