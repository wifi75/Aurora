import { beforeEach, describe, expect, it } from 'vitest';
import { normalize, overlaps, type DashSpecs } from './dashGrid';
import { SAT_LAYOUT_KEY, SAT_MINS, SAT_WIDGET_IDS, defaultLayout, loadLayout, parseLayout, saveLayout, serializeLayout } from './satLayout';

const specs: DashSpecs = { ...SAT_MINS, map: { ...SAT_MINS.map, hOf: (w) => Math.ceil(w * 0.6) } };

describe('default layout', () => {
  it('has every card once and respects the minimums', () => {
    const d = defaultLayout();
    expect(d.map((i) => i.id).sort()).toEqual([...SAT_WIDGET_IDS].sort());
    for (const i of d) {
      expect(i.w).toBeGreaterThanOrEqual(SAT_MINS[i.id as keyof typeof SAT_MINS].minW);
      expect(i.h).toBeGreaterThanOrEqual(SAT_MINS[i.id as keyof typeof SAT_MINS].minH);
    }
  });
  it('settles with no overlaps and nothing past column 12', () => {
    const d = normalize(defaultLayout(), specs);
    expect(d.every((a, i) => d.every((b, j) => i === j || !overlaps(a, b)))).toBe(true);
    expect(d.every((i) => i.x + i.w <= 12)).toBe(true);
  });
});

describe('parseLayout', () => {
  const good = () => JSON.parse(serializeLayout({ items: defaultLayout(), locked: true })) as Record<string, unknown>;
  it('accepts what serializeLayout writes', () => {
    const p = parseLayout(serializeLayout({ items: defaultLayout(), locked: true }));
    expect(p?.locked).toBe(true);
    expect(p?.items).toEqual(defaultLayout());
  });
  it('writes the height only when the user chose it', () => {
    const items = defaultLayout().map((i) => (i.id === 'ctl' ? { ...i, h: 12, userH: true } : i));
    const raw = JSON.parse(serializeLayout({ items, locked: false })) as { items: Record<string, unknown>[] };
    expect(raw.items.find((i) => i.id === 'ctl')?.h).toBe(12);
    expect(raw.items.find((i) => i.id === 'list')?.h).toBeUndefined();
    expect(parseLayout(JSON.stringify(raw))?.items.find((i) => i.id === 'ctl')).toMatchObject({ h: 12, userH: true });
    expect(parseLayout(JSON.stringify(raw))?.items.find((i) => i.id === 'list')?.userH).toBeUndefined();
  });
  it('rejects missing, broken and foreign data', () => {
    expect(parseLayout(null)).toBeNull();
    expect(parseLayout('')).toBeNull();
    expect(parseLayout('{not json')).toBeNull();
    expect(parseLayout('42')).toBeNull();
    expect(parseLayout('null')).toBeNull();
    expect(parseLayout(JSON.stringify({ ...good(), v: 99 }))).toBeNull();
    expect(parseLayout(JSON.stringify({ ...good(), items: 'x' }))).toBeNull();
  });
  it('rejects a layout with a new, missing, duplicated or invalid card', () => {
    const g = good();
    const items = g.items as Record<string, unknown>[];
    const withItems = (list: unknown[]) => JSON.stringify({ ...g, items: list });
    const patch = (p: Record<string, unknown> | null) => items.map((i, n) => (n === 0 ? (p === null ? null : { ...i, ...p }) : i));
    expect(parseLayout(withItems(items.slice(1)))).toBeNull();
    expect(parseLayout(withItems([...items.slice(1), { id: 'zz', x: 0, y: 0, w: 4, h: 4 }]))).toBeNull();
    expect(parseLayout(withItems([...items.slice(1), items[1]]))).toBeNull();
    expect(parseLayout(withItems(patch({ w: 1 })))).toBeNull();
    expect(parseLayout(withItems(patch({ x: 10 })))).toBeNull();
    expect(parseLayout(withItems(patch({ y: 1.5 })))).toBeNull();
    expect(parseLayout(withItems(patch({ h: '8' })))).toBeNull();
    expect(parseLayout(withItems(patch({ h: 1 })))).toBeNull();
    expect(parseLayout(withItems(patch(null)))).toBeNull();
  });
  it('treats a missing lock flag as unlocked', () => {
    const g = good();
    delete g.locked;
    expect(parseLayout(JSON.stringify(g))?.locked).toBe(false);
  });
});

describe('load and save', () => {
  beforeEach(() => window.localStorage.clear());
  it('returns the default when nothing is saved or the data is corrupt', () => {
    expect(loadLayout()).toEqual({ items: defaultLayout(), locked: false });
    window.localStorage.setItem(SAT_LAYOUT_KEY, 'garbage');
    expect(loadLayout().items).toEqual(defaultLayout());
  });
  it('round-trips', () => {
    const items = defaultLayout().map((i) => (i.id === 'list' ? { ...i, w: 3, x: 9 } : i));
    saveLayout({ items, locked: true });
    expect(loadLayout()).toEqual({ items, locked: true });
  });
  it('survives a browser storage that throws', () => {
    const real = window.localStorage;
    Object.defineProperty(window, 'localStorage', {
      configurable: true,
      get: () => {
        throw new Error('no');
      },
    });
    try {
      expect(loadLayout().items).toEqual(defaultLayout());
      expect(() => saveLayout({ items: defaultLayout(), locked: false })).not.toThrow();
    } finally {
      Object.defineProperty(window, 'localStorage', { configurable: true, value: real });
    }
  });
});
