import { act, render } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { SpectrumListener } from '../lib/spectrumFeed';

// il renderer vero (WebGPU/WebGL2) non c'e' nei test: un sostituto che conta le righe
const pushRow = vi.fn();
vi.mock('../render/backend', () => ({
  createBackend: () =>
    Promise.resolve({ kind: 'prova', setPalette: () => {}, setTone: () => {}, resize: () => {}, pushRow: (...args: unknown[]) => pushRow(...args), destroy: () => {} }),
}));
import { Waterfall } from './Waterfall';

/** Conta le letture delle misure degli elementi (quelle che possono forzare il ricalcolo dell'impaginazione). */
function countLayoutReads() {
  let reads = 0;
  const spies = [
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(() => {
      reads++;
      return { x: 0, y: 0, top: 0, left: 0, right: 800, bottom: 200, width: 800, height: 200, toJSON: () => ({}) } as DOMRect;
    }),
    ...(['offsetTop', 'offsetHeight', 'offsetWidth', 'clientHeight', 'clientWidth'] as const).map((name) =>
      vi.spyOn(HTMLElement.prototype, name, 'get').mockImplementation(() => {
        reads++;
        return 200;
      }),
    ),
  ];
  return { reads: () => reads, restore: () => spies.forEach((s) => s.mockRestore()) };
}

let observed: Array<() => void> = [];
beforeEach(() => {
  pushRow.mockClear();
  observed = [];
  vi.stubGlobal(
    'ResizeObserver',
    class {
      constructor(private readonly cb: () => void) {
        observed.push(() => this.cb());
      }
      observe() {}
      disconnect() {}
    },
  );
});
afterEach(() => vi.unstubAllGlobals());

describe('Waterfall', () => {
  it('does not read the layout for every row: the sizes come from the resize observer', async () => {
    let push: SpectrumListener | null = null;
    const register = (l: SpectrumListener) => {
      push = l;
      return () => {};
    };
    const counter = countLayoutReads();
    render(<Waterfall register={register} mode="FT8" />);
    await act(async () => {
      await Promise.resolve();
    });
    expect(push).not.toBeNull();
    const before = counter.reads();
    const bins = new Uint8Array(4096).fill(120);
    act(() => {
      for (let i = 0; i < 600; i++) push!(bins, 0, 3000);
    });
    const rows = pushRow.mock.calls.length;
    const perRow = (counter.reads() - before) / Math.max(1, rows);
    expect(rows).toBeGreaterThan(5);
    console.info(`cascata: ${rows} righe, ${(counter.reads() - before)} letture delle misure (${perRow.toFixed(2)} per riga)`);
    expect(perRow).toBe(0);
    // un ridimensionamento rilegge le misure una volta, non a ogni riga
    const atResize = counter.reads();
    act(() => observed.forEach((cb) => cb()));
    expect(counter.reads()).toBeGreaterThan(atResize);
    counter.restore();
  });
});
