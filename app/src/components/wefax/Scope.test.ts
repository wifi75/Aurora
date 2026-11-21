import { describe, expect, it } from 'vitest';
import { sqlBarY } from './Scope';

describe('sqlBarY', () => {
  it('puts 0 at the bottom and 100 at the top of the bar, with the same margin', () => {
    expect(sqlBarY(0, 100, 2)).toBe(98);
    expect(sqlBarY(100, 100, 2)).toBe(2);
    expect(sqlBarY(50, 100, 2)).toBe(50);
  });
  it('keeps the line inside the bar for out-of-range or broken values', () => {
    expect(sqlBarY(150, 100, 2)).toBe(2);
    expect(sqlBarY(-5, 100, 2)).toBe(98);
    expect(sqlBarY(Number.NaN, 100, 2)).toBe(98);
  });
  it('uses the same scale for the level and the threshold', () => {
    // piu' alto il valore, piu' in alto la riga (stessa funzione per il livello e per la soglia)
    expect(sqlBarY(60, 80, 2)).toBeLessThan(sqlBarY(30, 80, 2));
  });
});
