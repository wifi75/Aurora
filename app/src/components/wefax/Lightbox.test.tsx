import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { WefaxImageEntry } from '../../api/client';

vi.mock('../../api/client', () => ({
  postWefaxRotate: vi.fn(),
  postWefaxSlant: vi.fn(),
  wefaxImageUrl: (id: string) => `/api/v1/wefax/images/${id}`,
  wefaxAudioUrl: (id: string) => `/api/v1/wefax/images/${id}/audio`,
}));
import { postWefaxRotate } from '../../api/client';
import { Lightbox } from './Lightbox';

const ENTRY: WefaxImageEntry = { id: 'a1', file: 'a1.png', time_ms: 0, ioc: 576, width: 1800, height: 1200, complete: true } as WefaxImageEntry;

function open(entry: WefaxImageEntry = ENTRY, extra: { onClose?: () => void } = {}) {
  return render(<Lightbox entry={entry} onClose={extra.onClose ?? (() => undefined)} onDelete={() => undefined} onFixed={() => undefined} />);
}
const pct = () => screen.getByTestId('wefax-lb-pct').textContent;

beforeEach(() => {
  // il riquadro e' 900 x 600: la cartina 1800 x 1200 ci entra a meta'
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(900);
  vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockReturnValue(600);
  vi.stubGlobal('ResizeObserver', class { observe() {} unobserve() {} disconnect() {} });
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  delete (document.documentElement as { requestFullscreen?: unknown }).requestFullscreen;
});

describe('Lightbox WEFAX', () => {
  it('si apre adattata, tutta visibile', () => {
    open();
    expect(pct()).toBe('50 %');
    expect(screen.getByRole('button', { name: /fit|adatta/i, pressed: true })).toBeTruthy();
  });

  it('i tasti: + e - a gradini, 0 adatta, 1 pixel reali', () => {
    open();
    fireEvent.keyDown(window, { key: '+' });
    expect(pct()).toBe('67 %');
    fireEvent.keyDown(window, { key: '-' });
    expect(pct()).toBe('50 %');
    fireEvent.keyDown(window, { key: '1' });
    expect(pct()).toBe('100 %');
    fireEvent.keyDown(window, { key: '0' });
    expect(pct()).toBe('50 %');
  });

  it('i tasti non scattano mentre si scrive in un campo', () => {
    open();
    const box = screen.getByRole('checkbox');
    fireEvent.keyDown(box, { key: '1' });
    expect(pct()).toBe('50 %');
  });

  it('il doppio clic passa fra adatta e 1:1', () => {
    open();
    const area = screen.getByTestId('wefax-lb-scroll');
    fireEvent.doubleClick(area);
    expect(pct()).toBe('100 %');
    fireEvent.doubleClick(area);
    expect(pct()).toBe('50 %');
  });

  it('Esc chiude, R ruota a destra e si riadatta', async () => {
    const onClose = vi.fn();
    vi.mocked(postWefaxRotate).mockResolvedValue({ ...ENTRY, width: 1200, height: 1800 });
    open(ENTRY, { onClose });
    fireEvent.keyDown(window, { key: '1' });
    fireEvent.keyDown(window, { key: 'r' });
    await vi.waitFor(() => expect(postWefaxRotate).toHaveBeenCalledWith('a1', 'right'));
    await vi.waitFor(() => expect(pct()).toBe('50 %'));
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(onClose).toHaveBeenCalled();
  });

  it('senza l’API dello schermo intero il pulsante non c’e’', () => {
    open();
    expect(screen.queryByRole('button', { name: /full screen|schermo intero/i })).toBeNull();
  });

  it('con l’API dello schermo intero il pulsante c’e’', () => {
    (document.documentElement as { requestFullscreen?: unknown }).requestFullscreen = () => Promise.resolve();
    open();
    expect(screen.getByRole('button', { name: /full screen|schermo intero/i })).toBeTruthy();
  });

  it('il link dell’audio compare solo con has_audio', () => {
    const { unmount } = open();
    expect(screen.queryByRole('link', { name: /audio/i })).toBeNull();
    unmount();
    open({ ...ENTRY, has_audio: true, audio: 'a1.wav' });
    const a = screen.getByRole('link', { name: /audio/i });
    expect(a.getAttribute('href')).toBe('/api/v1/wefax/images/a1/audio');
    expect(a.getAttribute('download')).toBe('a1.wav');
  });

  it('mentre si riceve dice che la ricezione continua; senza ricezione non compare', () => {
    const { rerender } = render(<Lightbox entry={ENTRY} onClose={() => undefined} onDelete={() => undefined} onFixed={() => undefined} live={321} />);
    expect(screen.getByText(/321/)).toBeTruthy();
    rerender(<Lightbox entry={ENTRY} onClose={() => undefined} onDelete={() => undefined} onFixed={() => undefined} live={null} />);
    expect(screen.queryByText(/321/)).toBeNull();
  });
});
