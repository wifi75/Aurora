import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { SstvImageEntry, SstvModeInfo } from '../../api/client';

vi.mock('../../api/client', () => ({
  deleteSstvImage: vi.fn(() => Promise.resolve()),
  deleteSstvImages: vi.fn(() => Promise.resolve()),
  getSstvImageBlob: vi.fn(),
  getSstvImageCleanBlob: vi.fn(),
  sstvImageCleanUrl: vi.fn((id: string) => `/clean/${id}`),
  sstvImageUrl: vi.fn((id: string) => `/img/${id}`),
}));
import { History } from './History';

const MODES: SstvModeInfo[] = [{ id: 'm1', name: 'Martin 1', width: 320, height: 256, vis: 44, seconds: 114, color: 'rgb' }];
const img = (id: string, direction: string, call: string): SstvImageEntry => ({ id, file: `${id}.png`, direction, mode: 'm1', width: 320, height: 256, time_ms: 1_700_000_000_000, complete: true, call });
const IMAGES = [img('a', 'rx', 'IU3ABC'), img('b', 'rx', 'IU3ABD'), img('c', 'rx', 'IU3ABE'), img('t', 'tx', 'IU3TXX')];

function first<T>(a: T[]): T {
  const x = a[0];
  if (x === undefined) throw new Error('vuoto');
  return x;
}

function draw() {
  return render(<History images={IMAGES} modes={MODES} onChanged={() => undefined} onReuse={() => undefined} />);
}

describe('History: strisce con freccette', () => {
  let scrollBy: ReturnType<typeof vi.fn>;
  beforeEach(() => {
    scrollBy = vi.fn();
    Object.defineProperty(HTMLElement.prototype, 'scrollBy', { configurable: true, value: scrollBy });
  });
  afterEach(() => {
    const proto = HTMLElement.prototype as unknown as Record<string, unknown>;
    delete proto.scrollBy;
    delete proto.scrollWidth;
    delete proto.clientWidth;
  });

  it('con tutto in vista le freccette sono spente', () => {
    draw();
    for (const b of screen.getAllByLabelText(/Scorri a (sinistra|destra)/)) expect((b as HTMLButtonElement).disabled).toBe(true);
  });

  it("se c'e' altro da scorrere la freccia destra e' accesa e scorre di una pagina", () => {
    Object.defineProperty(HTMLElement.prototype, 'scrollWidth', { configurable: true, value: 1000 });
    Object.defineProperty(HTMLElement.prototype, 'clientWidth', { configurable: true, value: 400 });
    draw();
    const prev = first(screen.getAllByLabelText(/Scorri a sinistra/)) as HTMLButtonElement;
    const next = first(screen.getAllByLabelText(/Scorri a destra/)) as HTMLButtonElement;
    expect(prev.disabled).toBe(true);
    expect(next.disabled).toBe(false);
    fireEvent.click(next);
    expect(scrollBy).toHaveBeenCalledWith({ left: 340, behavior: 'smooth' });
  });

  it('le frecce della tastiera sulla striscia scorrono', () => {
    draw();
    fireEvent.keyDown(first(screen.getAllByRole('list')), { key: 'ArrowLeft' });
    expect(scrollBy).toHaveBeenCalled();
  });

  it('nella foto aperta in grande precedente e successiva, anche con i tasti', () => {
    draw();
    fireEvent.click(first(screen.getAllByRole('button', { name: /IU3ABD/ })));
    expect(screen.getByRole('dialog')).toBeTruthy();
    fireEvent.click(screen.getByLabelText(/Foto successiva/));
    expect(screen.getAllByText(/IU3ABE/).length).toBeGreaterThan(1);
    expect((screen.getByLabelText(/Foto successiva/) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.keyDown(window, { key: 'ArrowLeft' });
    fireEvent.keyDown(window, { key: 'ArrowLeft' });
    expect((screen.getByLabelText(/Foto precedente/) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByLabelText(/Foto successiva/) as HTMLButtonElement).disabled).toBe(false);
  });

  it('la foto trasmessa scorre solo fra le trasmesse', () => {
    draw();
    fireEvent.click(first(screen.getAllByRole('button', { name: /IU3TXX/ })));
    expect((screen.getByLabelText(/Foto successiva/) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByLabelText(/Foto precedente/) as HTMLButtonElement).disabled).toBe(true);
  });
});
