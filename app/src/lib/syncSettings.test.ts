import { describe, expect, it, vi } from 'vitest';
import { collect, deviceKind, hydrate } from './syncSettings';

function memory(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return {
    data,
    get length() {
      return Object.keys(data).length;
    },
    key: (i: number) => Object.keys(data)[i] ?? null,
    getItem: (k: string) => data[k] ?? null,
    setItem: (k: string, v: string) => {
      data[k] = v;
    },
  };
}

const reply = (body: unknown, ok = true) => Promise.resolve({ ok, json: () => Promise.resolve(body) } as Response);

describe('syncSettings', () => {
  it('tells the phone from the PC by width', () => {
    expect(deviceKind(390)).toBe('mobile');
    expect(deviceKind(1280)).toBe('desktop');
  });

  it('collects only the Aurora keys', () => {
    expect(collect(memory({ 'aurora-prefs': '1', altro: '2' }))).toEqual({ 'aurora-prefs': '1' });
  });

  it('brings the server preferences into the browser, the server wins', async () => {
    const store = memory({ 'aurora-prefs': 'vecchio' });
    const fetcher = vi.fn(() => reply({ 'aurora-prefs': 'dal-server', 'aurora-layout': '[]', estraneo: 'x' })) as unknown as typeof fetch;
    await hydrate(store, 'desktop', fetcher);
    expect(store.data).toEqual({ 'aurora-prefs': 'dal-server', 'aurora-layout': '[]' });
  });

  it('sends the browser preferences when the server has none yet', async () => {
    const store = memory({ 'aurora-prefs': 'mie' });
    const calls: [string, RequestInit | undefined][] = [];
    const fetcher = vi.fn((url: string, init?: RequestInit) => {
      calls.push([url, init]);
      return reply({});
    }) as unknown as typeof fetch;
    await hydrate(store, 'mobile', fetcher);
    const second = calls[1];
    expect(second?.[0]).toBe('/api/v1/ui-settings/mobile');
    expect(second?.[1]?.method).toBe('PUT');
    expect(JSON.parse(String(second?.[1]?.body))).toEqual({ 'aurora-prefs': 'mie' });
  });

  it('keeps what the browser has when the server cannot be reached', async () => {
    const store = memory({ 'aurora-prefs': 'mie' });
    await hydrate(store, 'desktop', (() => Promise.reject(new Error('rete'))) as unknown as typeof fetch);
    await hydrate(store, 'desktop', (() => reply({}, false)) as unknown as typeof fetch);
    expect(store.data).toEqual({ 'aurora-prefs': 'mie' });
  });
});
