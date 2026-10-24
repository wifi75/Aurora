import { beforeEach, describe, expect, it } from 'vitest';
import { COLOR_KEYS } from './colors';
import { defaultNotify, readNotify, saveNotify, useNotify } from './notify';

function memory(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return { data, getItem: (k: string) => data[k] ?? null, setItem: (k: string, v: string) => { data[k] = v; } };
}
const broken = { getItem: () => { throw new Error('no'); }, setItem: () => { throw new Error('no'); } };

describe('notification flags', () => {
  beforeEach(() => localStorage.clear());

  it('highlight every kind by default; only the rare ones make a sound', () => {
    const n = defaultNotify();
    for (const key of COLOR_KEYS) expect(n.highlight[key], key).toBe(true);
    expect(n.sound.new_dxcc).toBe(true);
    expect(n.sound.new_zone).toBe(true);
    expect(n.sound.cq).toBe(false);
    expect(n.sound.my_call).toBe(true);
    expect(n.sound.worked_call).toBe(false);
    expect(n.sound.auto).toBe(false);
  });

  it('are remembered and sanitised', () => {
    const store = memory();
    const n = defaultNotify();
    n.highlight.new_call = false;
    n.sound.cq = true;
    saveNotify(store, n);
    const back = readNotify(store);
    expect(back.highlight.new_call).toBe(false);
    expect(back.sound.cq).toBe(true);
    expect(back.highlight.new_dxcc).toBe(true);
    const raw = '{"highlight":{"new_dxcc":"no","new_grid":false,"inventato":false},"sound":7}';
    const odd = readNotify(memory({ 'aurora-notify': raw }));
    expect(odd.highlight.new_dxcc).toBe(true);
    expect(odd.highlight.new_grid).toBe(false);
    expect(odd.sound).toEqual(defaultNotify().sound);
    expect(readNotify(memory({ 'aurora-notify': 'non json' }))).toEqual(defaultNotify());
    expect(readNotify(broken)).toEqual(defaultNotify());
    expect(() => saveNotify(broken, n)).not.toThrow();
  });

  it('the shared store updates and persists at once', () => {
    useNotify.getState().setFlag('highlight', 'new_px', false);
    expect(useNotify.getState().notify.highlight.new_px).toBe(false);
    expect(JSON.parse(localStorage.getItem('aurora-notify') ?? '{}').highlight.new_px).toBe(false);
    useNotify.getState().reset();
    expect(useNotify.getState().notify).toEqual(defaultNotify());
  });
});
