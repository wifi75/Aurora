import { describe, expect, it } from 'vitest';
import { clampItem, compact, equalizeRows, fillRight, moveItem, normalize, overlaps, resizeItem, resizeWidth, snapCells, type DashItem, type DashSpecs } from './dashGrid';

const it_ = (id: string, x: number, y: number, w: number, h: number): DashItem => ({ id, x, y, w, h });
const specs: DashSpecs = {
  a: { minW: 2, minH: 2 },
  b: { minW: 3, minH: 3 },
  c: { minW: 3, minH: 6 },
  d: { minW: 2, minH: 2 },
  map: { minW: 4, minH: 2, hOf: (w) => Math.ceil(w / 2) + 1 },
};
const get = (items: readonly DashItem[], id: string) => items.find((i) => i.id === id) as DashItem;
const noOverlap = (items: readonly DashItem[]) => items.every((a, i) => items.every((b, j) => i === j || !overlaps(a, b)));
/** Nessuna cella libera dentro il rettangolo che contiene tutte le schede. */
const covered = (items: readonly DashItem[], cols = 12) => {
  const rows = Math.max(...items.map((i) => i.y + i.h));
  for (let y = 0; y < rows; y += 1) for (let x = 0; x < cols; x += 1) if (!items.some((i) => x >= i.x && x < i.x + i.w && y >= i.y && y < i.y + i.h)) return false;
  return true;
};

describe('overlaps', () => {
  it('touching edges do not overlap', () => {
    expect(overlaps(it_('a', 0, 0, 2, 2), it_('b', 2, 0, 2, 2))).toBe(false);
    expect(overlaps(it_('a', 0, 0, 2, 2), it_('b', 0, 2, 2, 2))).toBe(false);
    expect(overlaps(it_('a', 0, 0, 3, 3), it_('b', 2, 2, 2, 2))).toBe(true);
  });
});

describe('maxW', () => {
  const capped: DashSpecs = { a: { minW: 2, minH: 2, maxW: 4 }, b: { minW: 2, minH: 2 } };
  it('clamps the width of a capped card', () => {
    expect(clampItem(it_('a', 0, 0, 9, 3), capped).w).toBe(4);
  });
  it('fillRight does not widen a capped card past its maximum and gives the space to a wide one', () => {
    const r = fillRight([it_('a', 0, 0, 3, 3), it_('b', 6, 0, 3, 3)], capped);
    expect(get(r, 'a').w).toBeLessThanOrEqual(4);
    expect(get(r, 'b').x + get(r, 'b').w).toBe(12);
  });
  it('leaves free space empty when only capped cards are in the band', () => {
    const r = fillRight([it_('a', 0, 0, 3, 3)], capped);
    expect(get(r, 'a').w).toBe(4);
  });
  it('resizing cannot go past the maximum', () => {
    const r = resizeItem([it_('a', 0, 0, 3, 3)], 'a', 9, 3, capped);
    expect(get(r, 'a').w).toBe(4);
  });
});

describe('content rows', () => {
  it('an automatic card takes the height of its content, whatever the stored height', () => {
    const withRows: DashSpecs = { a: { minW: 2, minH: 2, rows: 6 } };
    expect(clampItem(it_('a', 0, 0, 4, 2), withRows).h).toBe(6);
    expect(clampItem(it_('a', 0, 0, 4, 20), withRows).h).toBe(6);
  });
  it('a height chosen by the user is kept, even lower than the content (it scrolls inside), never under the minimum', () => {
    const withRows: DashSpecs = { a: { minW: 2, minH: 3, rows: 8 } };
    expect(clampItem({ ...it_('a', 0, 0, 4, 5), userH: true }, withRows)).toEqual({ ...it_('a', 0, 0, 4, 5), userH: true });
    expect(clampItem({ ...it_('a', 0, 0, 4, 1), userH: true }, withRows).h).toBe(3);
    expect(clampItem({ ...it_('a', 0, 0, 4, 12), userH: true }, withRows).h).toBe(12);
  });
  it('resizing both ways keeps the chosen height as a user height', () => {
    const withRows: DashSpecs = { a: { minW: 2, minH: 3, rows: 8 }, b: { minW: 2, minH: 3, rows: 4 } };
    const r = resizeItem([it_('a', 0, 0, 4, 8), it_('b', 4, 0, 4, 4)], 'a', 5, 5, withRows);
    expect(get(r, 'a')).toEqual({ ...it_('a', 0, 0, 5, 5), userH: true });
    expect(get(r, 'b').h).toBe(4);
    expect(get(r, 'b').userH).toBeUndefined();
  });
  it('does not stretch cards by itself: a lower card keeps its content height next to a taller one', () => {
    const withRows: DashSpecs = { a: { minW: 2, minH: 2, rows: 4 }, b: { minW: 2, minH: 2, rows: 12 } };
    const r = normalize([it_('a', 0, 0, 6, 1), it_('b', 6, 0, 6, 1)], withRows);
    expect(get(r, 'a').h).toBe(4);
    expect(get(r, 'b').h).toBe(12);
  });
  it('equalises close content heights (3 rows or less) only with fill', () => {
    const withRows: DashSpecs = { a: { minW: 2, minH: 2, rows: 9 }, b: { minW: 2, minH: 2, rows: 11 } };
    const r = normalize([it_('a', 0, 0, 6, 1), it_('b', 6, 0, 6, 1)], withRows, { fill: true });
    expect(get(r, 'a').h).toBe(11);
    expect(get(r, 'b').h).toBe(11);
  });
});

describe('clampItem', () => {
  it('applies minimums, bounds and integers', () => {
    expect(clampItem(it_('b', -3, -2, 1, 1), specs)).toEqual(it_('b', 0, 0, 3, 3));
    expect(clampItem(it_('a', 11, 4.4, 5, 3), specs)).toEqual(it_('a', 7, 4, 5, 3));
    expect(clampItem(it_('a', 0, 0, 40, 3), specs)).toEqual(it_('a', 0, 0, 12, 3));
  });
  it('replaces non finite numbers', () => {
    expect(clampItem(it_('a', Number.NaN, Number.POSITIVE_INFINITY, Number.NaN, Number.NaN), specs)).toEqual(it_('a', 0, 0, 2, 2));
  });
  it('gives a fixed-ratio card the height of its width', () => {
    expect(clampItem(it_('map', 0, 0, 8, 99), specs)).toEqual(it_('map', 0, 0, 8, 5));
    expect(clampItem(it_('map', 0, 0, 1, 1), specs).w).toBe(4);
  });
  it('treats an unknown card as 1x1 minimum', () => {
    expect(clampItem(it_('zz', 3, 3, 0, 0), specs)).toEqual(it_('zz', 3, 3, 1, 1));
  });
});

describe('compact', () => {
  it('lifts cards until they touch another or the top', () => {
    const r = compact([it_('a', 0, 5, 4, 2), it_('b', 4, 3, 4, 2), it_('c', 0, 9, 4, 2)]);
    expect(get(r, 'a').y).toBe(0);
    expect(get(r, 'b').y).toBe(0);
    expect(get(r, 'c').y).toBe(2);
  });
  it('pushes down a card that overlaps another', () => {
    const r = compact([it_('a', 0, 0, 4, 3), it_('b', 2, 1, 4, 2)]);
    expect(get(r, 'a').y).toBe(0);
    expect(get(r, 'b').y).toBe(3);
    expect(noOverlap(r)).toBe(true);
  });
  it('keeps the first card where it is and adapts the others', () => {
    const r = compact([it_('a', 0, 0, 4, 3), it_('b', 0, 3, 4, 3)], 'b');
    expect(get(r, 'b').y).toBe(3);
    expect(get(r, 'a').y).toBe(0);
    const r2 = compact([it_('a', 0, 0, 4, 3), it_('b', 0, 1, 4, 3)], 'b');
    expect(get(r2, 'b').y).toBe(1);
    expect(get(r2, 'a').y).toBe(4);
  });
  it('is idempotent and keeps the order of the input', () => {
    const start = [it_('b', 4, 3, 4, 2), it_('a', 0, 5, 4, 2)];
    const r = compact(start);
    expect(r.map((i) => i.id)).toEqual(['b', 'a']);
    expect(compact(r)).toEqual(r);
  });
});

describe('fillRight', () => {
  it('widens the last card of a band up to column 12', () => {
    const r = fillRight([it_('a', 0, 0, 4, 3), it_('b', 4, 0, 4, 3)], specs);
    expect(get(r, 'a')).toEqual(it_('a', 0, 0, 4, 3));
    expect(get(r, 'b')).toEqual(it_('b', 4, 0, 8, 3));
  });
  it('splits free columns between two cards', () => {
    const r = fillRight([it_('a', 0, 0, 3, 3), it_('b', 7, 0, 3, 3)], specs);
    expect(get(r, 'a').w + get(r, 'b').w).toBe(12);
    expect(get(r, 'a').x + get(r, 'a').w).toBe(get(r, 'b').x);
    expect(get(r, 'b').x + get(r, 'b').w).toBe(12);
    expect(Math.abs(get(r, 'a').w - get(r, 'b').w)).toBeLessThanOrEqual(2);
  });
  it('does not widen over a card below or above', () => {
    const r = fillRight([it_('a', 0, 0, 4, 3), it_('b', 4, 0, 4, 6), it_('c', 8, 0, 4, 3), it_('d', 8, 3, 2, 3)], specs);
    expect(noOverlap(r)).toBe(true);
    expect(get(r, 'd').x + get(r, 'd').w).toBe(12);
  });
  it('leaves fixed-ratio cards alone and lets a neighbour take the gap', () => {
    const r = fillRight([it_('map', 0, 0, 6, 4), it_('a', 8, 0, 4, 4)], specs);
    expect(get(r, 'map').w).toBe(6);
    expect(get(r, 'a')).toEqual(it_('a', 6, 0, 6, 4));
  });
  it('lets a card take the gap left by a fixed-ratio card on its right', () => {
    const r = fillRight([it_('a', 0, 0, 4, 4), it_('map', 6, 0, 6, 4)], specs);
    expect(get(r, 'a').w).toBe(6);
  });
  it('widens the active card only after its neighbours', () => {
    const r = fillRight([it_('a', 0, 0, 4, 3), it_('b', 6, 0, 3, 3)], specs, 12, 'a');
    // b prende prima le colonne a destra e a sinistra libere, poi a ricevere cio' che resta e' a
    expect(get(r, 'b').x + get(r, 'b').w).toBe(12);
    expect(get(r, 'a').x + get(r, 'a').w).toBe(get(r, 'b').x);
  });
  it('does nothing on a full band', () => {
    const start = [it_('a', 0, 0, 5, 3), it_('b', 5, 0, 7, 3)];
    expect(fillRight(start, specs)).toEqual(start);
  });
});

describe('equalizeRows', () => {
  it('gives the same height to cards of the same band, the tallest decides', () => {
    const r = equalizeRows([it_('a', 0, 0, 4, 5), it_('b', 4, 0, 4, 7), it_('d', 8, 0, 4, 6)], specs);
    expect(r.map((i) => i.h)).toEqual([7, 7, 7]);
  });
  it('does not stretch a card that is much lower than its neighbour (more than 3 rows)', () => {
    const r = equalizeRows([it_('a', 0, 0, 4, 3), it_('b', 4, 0, 4, 7)], specs);
    expect(r.map((i) => i.h)).toEqual([3, 7]);
    const r2 = equalizeRows([it_('a', 0, 0, 4, 4), it_('b', 4, 0, 4, 7)], specs);
    expect(r2.map((i) => i.h)).toEqual([7, 7]);
  });
  it('lets the priority card decide', () => {
    const r = equalizeRows([it_('a', 0, 0, 4, 5), it_('b', 4, 0, 4, 7)], specs, 'a');
    expect(r.map((i) => i.h)).toEqual([5, 5]);
  });
  it('respects the minimum height of each card', () => {
    const r = equalizeRows([it_('a', 0, 0, 4, 2), it_('c', 4, 0, 4, 6)], specs, 'a');
    expect(get(r, 'c').h).toBe(6);
  });
  it('leaves fixed-ratio cards and cards of other bands alone', () => {
    const r = equalizeRows([it_('map', 0, 0, 8, 5), it_('a', 8, 0, 4, 3), it_('b', 8, 3, 4, 8)], specs);
    expect(get(r, 'map').h).toBe(5);
    expect(get(r, 'a').h).toBe(3);
    expect(get(r, 'b').h).toBe(8);
  });
});

describe('moveItem', () => {
  const base = [it_('a', 0, 0, 6, 4), it_('b', 6, 0, 6, 4)];
  it('moves into free space and compacts', () => {
    const r = moveItem([it_('a', 0, 0, 6, 3), it_('b', 6, 0, 6, 3), it_('c', 0, 3, 6, 6)], 'c', 6, 20, specs);
    expect(noOverlap(r)).toBe(true);
    expect(get(r, 'c').y).toBe(3);
  });
  it('pushes the cards in the target cells downwards', () => {
    const r = moveItem([it_('a', 0, 0, 12, 3), it_('b', 0, 3, 6, 4), it_('c', 6, 3, 6, 4), it_('d', 0, 7, 12, 3)], 'd', 3, 3, specs);
    expect(get(r, 'd').y).toBe(3);
    expect(get(r, 'b').y).toBeGreaterThanOrEqual(get(r, 'd').y + get(r, 'd').h);
    expect(get(r, 'c').y).toBeGreaterThanOrEqual(get(r, 'd').y + get(r, 'd').h);
    expect(noOverlap(r)).toBe(true);
  });
  it('swaps two side by side cards when dropped on the other', () => {
    const r = moveItem(base, 'a', 6, 0, specs);
    expect(get(r, 'a').x).toBe(6);
    expect(get(r, 'b').x).toBe(0);
    expect(noOverlap(r)).toBe(true);
    expect(covered(r)).toBe(true);
  });
  it('keeps the card inside the grid', () => {
    const r = moveItem(base, 'a', 40, -5, specs);
    expect(get(r, 'a').x + get(r, 'a').w).toBeLessThanOrEqual(12);
    expect(get(r, 'a').y).toBeGreaterThanOrEqual(0);
  });
  it('ignores an unknown card', () => {
    expect(moveItem(base, 'zz', 3, 3, specs)).toEqual(normalize(base, specs));
  });
  it('does not mutate the input', () => {
    const copy = JSON.parse(JSON.stringify(base)) as DashItem[];
    moveItem(base, 'a', 6, 5, specs);
    expect(base).toEqual(copy);
  });
});

describe('resizeItem', () => {
  it('widens a card and pushes the neighbour down, without widening anything else', () => {
    const r = resizeItem([it_('a', 0, 0, 6, 4), it_('b', 6, 0, 6, 4)], 'a', 9, 4, specs);
    expect(get(r, 'a').w).toBe(9);
    expect(get(r, 'b')).toEqual(it_('b', 6, 4, 6, 4));
    expect(noOverlap(r)).toBe(true);
  });
  it('makes a taller card push the cards below', () => {
    const r = resizeItem([it_('a', 0, 0, 6, 3), it_('d', 0, 3, 6, 3)], 'a', 6, 6, specs);
    expect(get(r, 'a').h).toBe(6);
    expect(get(r, 'd').y).toBe(6);
  });
  it('does not go under the minimum or past the right edge', () => {
    const r = resizeItem([it_('b', 8, 0, 4, 4)], 'b', 1, 1, specs);
    expect(get(r, 'b').w).toBeGreaterThanOrEqual(3);
    expect(get(r, 'b').h).toBeGreaterThanOrEqual(3);
    const r2 = resizeItem([it_('a', 8, 0, 4, 4)], 'a', 9, 4, specs);
    expect(get(r2, 'a').x + get(r2, 'a').w).toBeLessThanOrEqual(12);
  });
  it('does not equalise or stretch neighbours by default', () => {
    const r = resizeItem([it_('a', 0, 0, 6, 5), it_('d', 6, 0, 6, 5)], 'a', 6, 3, specs);
    expect(get(r, 'a').h).toBe(3);
    expect(get(r, 'd').h).toBe(5);
  });
  it('keeps the fixed ratio: height follows width, free space beside it stays free', () => {
    const start = [it_('map', 0, 0, 8, 5), it_('a', 8, 0, 4, 5)];
    const r = resizeItem(start, 'map', 6, 40, specs);
    expect(get(r, 'map').w).toBe(6);
    expect(get(r, 'map').h).toBe(4);
    expect(get(r, 'a')).toEqual(it_('a', 8, 0, 4, 5));
    const r2 = resizeItem(start, 'map', 12, 2, specs);
    expect(get(r2, 'map').h).toBe(7);
    expect(get(r2, 'a').y).toBe(7);
  });
  it('ignores an unknown card', () => {
    const base = [it_('a', 0, 0, 6, 4)];
    expect(resizeItem(base, 'zz', 3, 3, specs)).toEqual(normalize(base, specs));
  });
});

describe('resizeWidth and snapCells', () => {
  it('changes only the width, in whole columns, inside min and max', () => {
    const capped: DashSpecs = { a: { minW: 3, minH: 2, maxW: 5 }, b: { minW: 2, minH: 2 } };
    const start = [it_('a', 0, 0, 4, 6), it_('b', 4, 0, 4, 3)];
    const r = resizeWidth(start, 'a', 5, capped);
    expect(get(r, 'a')).toMatchObject(it_('a', 0, 0, 5, 6));
    expect(get(r, 'b').y).toBe(6);
    expect(get(resizeWidth(start, 'a', 9, capped), 'a').w).toBe(5);
    expect(get(resizeWidth(start, 'a', 1, capped), 'a').w).toBe(3);
    expect(get(resizeWidth(start, 'a', 3, capped), 'a').h).toBe(6);
  });
  it('shrinking leaves the space free: neighbours are not widened', () => {
    const r = resizeWidth([it_('a', 0, 0, 6, 4), it_('b', 6, 0, 4, 4)], 'a', 3, specs);
    expect(get(r, 'a').w).toBe(3);
    expect(get(r, 'b')).toEqual(it_('b', 6, 0, 4, 4));
  });
  it('snaps pixels to the nearest column', () => {
    expect(snapCells(100, 90, 10)).toBe(1);
    expect(snapCells(190, 90, 10)).toBe(2);
    expect(snapCells(240, 90, 10)).toBe(3);
    expect(snapCells(280, 90, 10)).toBe(3);
    expect(snapCells(-50, 90, 10)).toBe(1);
  });
});

describe('normalize', () => {
  it('does not widen or stretch anything by default, only compacts', () => {
    const r = normalize([it_('a', 0, 5, 3, 3), it_('b', 6, 9, 3, 4)], specs);
    expect(r).toEqual([it_('a', 0, 0, 3, 3), it_('b', 6, 0, 3, 4)]);
  });
  it('with fill: fills the page with no holes, whatever the starting point', () => {
    const r = normalize([it_('map', 0, 0, 6, 1), it_('a', 9, 4, 2, 2), it_('b', 3, 12, 3, 3), it_('c', 5, 30, 3, 6), it_('d', 0, 2, 4, 2)], specs, { fill: true });
    expect(noOverlap(r)).toBe(true);
    for (const i of r) expect(i.x + i.w).toBeLessThanOrEqual(12);
    // nessun vuoto sopra una scheda
    for (const i of r) {
      if (i.y === 0) continue;
      expect(r.some((o) => o !== i && overlaps({ ...i, y: i.y - 1 }, o))).toBe(true);
    }
  });
  it('is idempotent', () => {
    const r = normalize([it_('map', 0, 0, 8, 1), it_('a', 8, 0, 4, 3), it_('b', 8, 3, 4, 3), it_('c', 0, 5, 6, 6), it_('d', 6, 5, 6, 6)], specs, { fill: true });
    expect(normalize(r, specs, { fill: true })).toEqual(r);
  });
  it('repairs a broken layout (overlaps, out of bounds, tiny sizes)', () => {
    const r = normalize([it_('a', 50, -4, 0, 0), it_('b', 50, -4, 0, 0), it_('c', 3, 3, 99, 99)], specs);
    expect(noOverlap(r)).toBe(true);
    for (const i of r) {
      expect(i.x).toBeGreaterThanOrEqual(0);
      expect(i.x + i.w).toBeLessThanOrEqual(12);
      expect(i.w).toBeGreaterThanOrEqual(specs[i.id]?.minW ?? 1);
      expect(i.h).toBeGreaterThanOrEqual(specs[i.id]?.minH ?? 1);
    }
  });
  it('handles an empty layout and a single card', () => {
    expect(normalize([], specs)).toEqual([]);
    expect(normalize([it_('b', 5, 5, 3, 3)], specs)).toEqual([it_('b', 5, 0, 3, 3)]);
    expect(normalize([it_('b', 5, 5, 3, 3)], specs, { fill: true })).toEqual([it_('b', 0, 0, 12, 3)]);
  });
  it('keeps a fixed-ratio card at exactly its ratio height after any change', () => {
    const r = normalize([it_('map', 0, 0, 7, 99), it_('a', 7, 0, 5, 3)], specs);
    expect(get(r, 'map').h).toBe(Math.ceil(7 / 2) + 1);
  });
  it('covers the whole width of every row of a typical layout', () => {
    const r = normalize([it_('map', 0, 0, 8, 5), it_('a', 8, 0, 4, 3), it_('b', 8, 3, 4, 3), it_('c', 0, 5, 8, 6)], specs, { fill: true });
    expect(noOverlap(r)).toBe(true);
    for (let y = 0; y < 5; y += 1) {
      const widths = r.filter((i) => y >= i.y && y < i.y + i.h).reduce((s, i) => s + i.w, 0);
      expect(widths).toBe(12);
    }
  });
});
