import { describe, expect, it } from 'vitest';
import { clearHeight, defaultLayout, movePanel, readLayout, resizePanel, saveLayout, setSplit } from './layout';

function memory(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return { data, getItem: (k: string) => data[k] ?? null, setItem: (k: string, v: string) => { data[k] = v; } };
}
const broken = { getItem: () => { throw new Error('no'); }, setItem: () => { throw new Error('no'); } };

describe('layout', () => {
  it('starts like JTDX: all the stations on the left with the waterfall under them', () => {
    const l = defaultLayout();
    expect(l.left).toEqual(['all', 'waterfall']);
    expect(l.right).toEqual(['station', 'cq']);
  });

  it('moves a panel to another zone at a position, keeping every panel exactly once', () => {
    const l = movePanel(defaultLayout(), 'cq', 'left', 0);
    expect(l.left).toEqual(['cq', 'all', 'waterfall']);
    expect(l.right).toEqual(['station']);
  });

  it('reorders inside the same zone and clamps the position', () => {
    expect(movePanel(defaultLayout(), 'station', 'right', 2).right).toEqual(['cq', 'station']);
    expect(movePanel(defaultLayout(), 'station', 'right', 99).right).toEqual(['cq', 'station']);
    expect(movePanel(defaultLayout(), 'all', 'right', -5).right).toEqual(['all', 'station', 'cq']);
  });

  it('puts the waterfall below everything else', () => {
    const l = movePanel(movePanel(defaultLayout(), 'waterfall', 'right', 0), 'waterfall', 'left', 5);
    expect(l.left).toEqual(['all', 'waterfall']);
  });

  it('lets a zone stay empty', () => {
    let l = defaultLayout();
    for (const id of ['station', 'cq'] as const) l = movePanel(l, id, 'left', 0);
    expect(l.right).toEqual([]);
  });

  it('forgets a chosen height', () => {
    const l = clearHeight(resizePanel(defaultLayout(), 'cq', 300), 'cq');
    expect(l.heights.cq).toBeUndefined();
  });

  it('clamps sizes and split', () => {
    expect(resizePanel(defaultLayout(), 'cq', 5).heights.cq).toBe(80);
    expect(resizePanel(defaultLayout(), 'cq', 99999).heights.cq).toBe(2000);
    expect(resizePanel(defaultLayout(), 'cq', 300.4).heights.cq).toBe(300);
    expect(setSplit(defaultLayout(), 0.01).split).toBe(0.25);
    expect(setSplit(defaultLayout(), 3).split).toBe(0.85);
  });

  it('remembers the layout and survives garbage or a broken storage', () => {
    const store = memory();
    const l = resizePanel(movePanel(defaultLayout(), 'cq', 'left', 0), 'cq', 321);
    saveLayout(store, l);
    expect(readLayout(store)).toEqual(l);
    expect(readLayout(memory({ 'aurora-layout': 'non json' }))).toEqual(defaultLayout());
    expect(readLayout(memory({ 'aurora-layout': '{"left":["waterfall"],"right":["station"],"heights":{},"split":0.5}' }))).toEqual(defaultLayout());
    expect(readLayout(memory({ 'aurora-layout': '{"left":["waterfall","waterfall"],"right":["station","cq","all"],"heights":{},"split":0.5}' }))).toEqual(defaultLayout());
    expect(readLayout(broken)).toEqual(defaultLayout());
    expect(() => saveLayout(broken, l)).not.toThrow();
    expect(readLayout(null)).toEqual(defaultLayout());
  });
});
