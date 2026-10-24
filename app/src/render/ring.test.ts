import { describe, expect, it } from 'vitest';
import { RowRing } from './ring';

const row = (...v: number[]) => Uint8Array.from(v);

describe('RowRing', () => {
  it('wraps and overwrites oldest', () => {
    const ring = new RowRing(1, 3);
    for (const v of [1, 2, 3, 4]) ring.push(row(v));
    expect(Array.from(ring.data)).toEqual([4, 2, 3]);
  });

  it('head and filled after more pushes than height', () => {
    const ring = new RowRing(1, 3);
    expect([ring.head, ring.filled]).toEqual([0, 0]);
    ring.push(row(1));
    expect([ring.head, ring.filled]).toEqual([1, 1]);
    for (const v of [2, 3, 4, 5]) ring.push(row(v));
    expect(ring.filled).toBe(3);
    expect(ring.head).toBe(2);
  });

  it('resamples a wrong-width row', () => {
    const ring = new RowRing(4, 2);
    ring.push(row(10, 20));
    expect(Array.from(ring.data.slice(0, 4))).toEqual([10, 10, 20, 20]);
    ring.push(row(1, 2, 3, 4, 5, 6, 7, 8));
    expect(Array.from(ring.data.slice(4, 8))).toEqual([1, 3, 5, 7]);
  });

  it('push returns the index of the row it wrote', () => {
    const ring = new RowRing(1, 3);
    expect([ring.push(row(1)), ring.push(row(2)), ring.push(row(3)), ring.push(row(4))]).toEqual([0, 1, 2, 0]);
  });

  it('copyInto writes newest row first', () => {
    const ring = new RowRing(1, 3);
    for (const v of [1, 2, 3, 4]) ring.push(row(v));
    const out = new Uint8Array(3);
    ring.copyInto(out);
    expect(Array.from(out)).toEqual([4, 3, 2]);
  });

  it('ignores an empty row', () => {
    const ring = new RowRing(2, 2);
    expect(ring.push(new Uint8Array(0))).toBe(-1);
    expect(ring.filled).toBe(0);
  });

  it('rejects non-positive dimensions', () => {
    expect(() => new RowRing(0, 4)).toThrow();
    expect(() => new RowRing(4, 0)).toThrow();
  });
});
