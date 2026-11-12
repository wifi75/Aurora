import { describe, expect, it } from 'vitest';
import { modeBase, noteTrPeriod, periodPosition, periodSecs } from './period';

const at = (h: number, m: number, s: number, ms = 0) => Date.UTC(2026, 9, 24, h, m, s, ms);

describe('period position', () => {
  it('shows the elapsed whole seconds like the old one: 13:02:14 is 14/15', () => {
    expect(periodPosition(at(13, 2, 14), 'FT8').seconds).toBe(14);
    expect(periodPosition(at(13, 2, 14, 999), 'FT8').seconds).toBe(14);
  });

  it('starts a new period on :00, :15, :30 and :45', () => {
    for (const s of [0, 15, 30, 45]) {
      const p = periodPosition(at(13, 5, s), 'FT8');
      expect(p.seconds).toBe(0);
      expect(p.fraction).toBe(0);
    }
    expect(periodPosition(at(13, 5, 59, 999), 'FT8').seconds).toBe(14);
  });

  it('moves the bar smoothly between whole seconds', () => {
    expect(periodPosition(at(13, 0, 7, 500), 'FT8').fraction).toBeCloseTo(0.5, 5);
  });

  it('uses 7.5 seconds for FT4', () => {
    expect(periodSecs('FT4')).toBe(7.5);
    expect(periodPosition(at(13, 0, 3, 750), 'FT4').fraction).toBeCloseTo(0.5, 5);
    expect(periodPosition(at(13, 0, 7, 400), 'FT4').seconds).toBe(7);
  });
});

describe('JT65 and JT9 periods', () => {
  it('last a whole minute, so the progress bar counts to 60', () => {
    expect(periodSecs('JT65')).toBe(60);
    expect(periodSecs('JT9')).toBe(60);
    expect(periodSecs('FT8')).toBe(15);
    expect(periodSecs('FT4')).toBe(7.5);
    const at = Date.UTC(2026, 9, 26, 20, 36, 42);
    expect(periodPosition(at, 'JT65')).toMatchObject({ seconds: 42, length: 60 });
  });
});

describe('MSK144 period', () => {
  it('follows the T/R period reported by the engine and ignores other values and modes', () => {
    expect(periodSecs('MSK144')).toBe(15);
    noteTrPeriod('MSK144', 30);
    expect(periodSecs('MSK144')).toBe(30);
    expect(periodPosition(at(13, 0, 45), 'MSK144').seconds).toBe(15);
    noteTrPeriod('MSK144', 20);
    noteTrPeriod('FT8', 5);
    expect(periodSecs('MSK144')).toBe(30);
    expect(periodSecs('FT8')).toBe(15);
    noteTrPeriod('MSK144', 15);
  });
});

describe('WSPR period', () => {
  it('lasts two minutes starting on even minutes', () => {
    expect(periodSecs('WSPR')).toBe(120);
    expect(periodPosition(at(13, 1, 30), 'WSPR').seconds).toBe(90);
    expect(periodPosition(at(13, 2, 0), 'WSPR').seconds).toBe(0);
  });
});

describe('FST4 and FST4W periods', () => {
  it('reads the period from the mode name reported by the engine', () => {
    expect(periodSecs('FST4-15')).toBe(15);
    expect(periodSecs('FST4-60')).toBe(60);
    expect(periodSecs('FST4-1800')).toBe(1800);
    expect(periodSecs('FST4W-120')).toBe(120);
    expect(periodSecs('fst4w-900')).toBe(900);
    expect(periodPosition(at(13, 3, 30), 'FST4W-300').seconds).toBe(210);
  });

  it('falls back to the WSJT-X defaults without a valid period', () => {
    expect(periodSecs('FST4')).toBe(60);
    expect(periodSecs('FST4W')).toBe(120);
    expect(periodSecs('FST4-45')).toBe(60);
    expect(periodSecs('FST4W-15')).toBe(120);
  });

  it('strips the period only from FST4 and FST4W', () => {
    expect(modeBase('FST4-60')).toBe('FST4');
    expect(modeBase('FST4W-1800')).toBe('FST4W');
    expect(modeBase('FST4')).toBe('FST4');
    expect(modeBase('FT8')).toBe('FT8');
    expect(modeBase('Q65-30A')).toBe('Q65-30A');
  });
});
