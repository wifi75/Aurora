import { describe, expect, it } from 'vitest';
import { clearHeight, defaultLayout, movePanel, toggleCollapsed, readLayout, resizePanel, saveLayout, setSplit } from './layout';

function memory(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return { data, getItem: (k: string) => data[k] ?? null, setItem: (k: string, v: string) => { data[k] = v; } };
}
const broken = { getItem: () => { throw new Error('no'); }, setItem: () => { throw new Error('no'); } };

describe('layout', () => {
  it('starts like JTDX: all the stations on the left with the waterfall under them', () => {
    const l = defaultLayout();
    expect(l.left).toEqual(['all', 'log', 'waterfall']);
    expect(l.right).toEqual(['cluster', 'station', 'tx', 'cq']);
  });

  it('moves a panel to another zone at a position, keeping every panel exactly once', () => {
    const l = movePanel(defaultLayout(), 'cq', 'left', 0);
    expect(l.left).toEqual(['cq', 'all', 'log', 'waterfall']);
    expect(l.right).toEqual(['cluster', 'station', 'tx']);
  });

  it('reorders inside the same zone and clamps the position', () => {
    expect(movePanel(defaultLayout(), 'station', 'right', 2).right).toEqual(['cluster', 'tx', 'station', 'cq']);
    expect(movePanel(defaultLayout(), 'station', 'right', 99).right).toEqual(['cluster', 'tx', 'cq', 'station']);
    expect(movePanel(defaultLayout(), 'all', 'right', -5).right).toEqual(['all', 'cluster', 'station', 'tx', 'cq']);
  });

  it('puts the waterfall below everything else', () => {
    const l = movePanel(movePanel(defaultLayout(), 'waterfall', 'right', 0), 'waterfall', 'left', 5);
    expect(l.left).toEqual(['all', 'log', 'waterfall']);
  });

  it('lets a zone stay empty', () => {
    let l = defaultLayout();
    for (const id of ['cluster', 'station', 'tx', 'cq'] as const) l = movePanel(l, id, 'left', 0);
    expect(l.right).toEqual([]);
  });

  it('collapses a panel to its title bar and expands it again', () => {
    const once = toggleCollapsed(defaultLayout(), 'waterfall');
    expect(once.collapsed).toEqual(['waterfall']);
    expect(toggleCollapsed(once, 'waterfall').collapsed).toEqual([]);
  });

  it('remembers collapsed panels and ignores bad values', () => {
    const store = memory();
    saveLayout(store, toggleCollapsed(defaultLayout(), 'cq'));
    expect(readLayout(store).collapsed).toEqual(['cq']);
    const raw = '{"left":["all","waterfall"],"right":["station","tx","cq"],"heights":{},"split":0.5,"collapsed":["cq","nope",7]}';
    expect(readLayout(memory({ 'aurora-layout': raw })).collapsed).toEqual(['cq']);
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
    expect(readLayout(memory({ 'aurora-layout': '{"left":["waterfall","waterfall"],"right":["station","tx","cq","all"],"heights":{},"split":0.5}' }))).toEqual(defaultLayout());
    expect(readLayout(broken)).toEqual(defaultLayout());
    expect(() => saveLayout(broken, l)).not.toThrow();
    expect(readLayout(null)).toEqual(defaultLayout());
  });
});

describe('layout saved before the Log window existed', () => {
  it('gets the Log window right next to the band activity, keeping the rest', () => {
    const old = { left: ['all', 'waterfall'], right: ['station', 'tx', 'cq'], heights: {}, split: 0.6, collapsed: [] };
    const l = readLayout(memory({ 'aurora-layout': JSON.stringify(old) }));
    expect(l.left).toEqual(['all', 'log', 'waterfall']);
    expect(l.right).toEqual(['cluster', 'station', 'tx', 'cq']);
  });
});

describe('layout saved before the DX Cluster window existed', () => {
  it('puts the DX Cluster window on the left of the commands, in the same column', () => {
    const old = { left: ['all', 'log', 'waterfall'], right: ['station', 'tx', 'cq'], heights: {}, split: 0.6, collapsed: [] };
    const l = readLayout(memory({ 'aurora-layout': JSON.stringify(old) }));
    expect(l.right).toEqual(['cluster', 'station', 'tx', 'cq']);
    expect(l.left).toEqual(['all', 'log', 'waterfall']);
    expect(l.heights.cluster).toBe(340);
  });

  it('puts it at the top of the column when the commands were moved elsewhere', () => {
    const old = { left: ['all', 'log', 'waterfall', 'station'], right: ['tx', 'cq'], heights: {}, split: 0.6, collapsed: [] };
    expect(readLayout(memory({ 'aurora-layout': JSON.stringify(old) })).right).toEqual(['cluster', 'tx', 'cq']);
  });

  it('adds both new windows to a layout that is older than both', () => {
    const old = { left: ['all', 'waterfall'], right: ['station', 'tx', 'cq'], heights: {}, split: 0.6, collapsed: [] };
    const l = readLayout(memory({ 'aurora-layout': JSON.stringify(old) }));
    expect(l.left).toEqual(['all', 'log', 'waterfall']);
    expect(l.right).toEqual(['cluster', 'station', 'tx', 'cq']);
  });
});
