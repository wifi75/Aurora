import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { emitSpectrum, spectrumListenerCount } from '../lib/spectrumFeed';

// il renderer vero (WebGPU/WebGL2) non c'e' nei test: un sostituto che si abbona alle righe come la cascata
vi.mock('../components/Waterfall', async () => {
  const { useEffect } = await import('react');
  return {
    Waterfall: ({ register }: { register(l: () => void): () => void }) => {
      useEffect(() => register(() => {}), [register]);
      return <div data-testid="wf-renderer" />;
    },
  };
});
import { WaterfallPreview } from './WaterfallPreview';

function setVisibility(state: 'visible' | 'hidden') {
  Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => state });
  act(() => {
    document.dispatchEvent(new Event('visibilitychange'));
  });
}

beforeEach(() => {
  localStorage.clear();
  setVisibility('visible');
});
afterEach(() => vi.useRealTimers());

describe('WaterfallPreview', () => {
  it('is off until the button is pressed and remembers the choice', () => {
    const { unmount } = render(<WaterfallPreview />);
    expect(screen.queryByTestId('wf-renderer')).toBeNull();
    expect(spectrumListenerCount()).toBe(0);
    fireEvent.click(screen.getByRole('button', { name: 'Mostra anteprima dal vivo' }));
    expect(screen.getByTestId('wf-renderer')).toBeInTheDocument();
    expect(spectrumListenerCount()).toBe(2);
    unmount();
    render(<WaterfallPreview />);
    expect(screen.getByTestId('wf-renderer')).toBeInTheDocument();
  });

  it('unsubscribes everything when the tab is left, closed or the window hidden', () => {
    localStorage.setItem('aurora.wfPreview.open', '1');
    const { unmount } = render(<WaterfallPreview />);
    expect(spectrumListenerCount()).toBe(2);
    setVisibility('hidden');
    expect(screen.queryByTestId('wf-renderer')).toBeNull();
    expect(spectrumListenerCount()).toBe(0);
    setVisibility('visible');
    expect(spectrumListenerCount()).toBe(2);
    fireEvent.click(screen.getByRole('button', { name: 'Nascondi anteprima' }));
    expect(spectrumListenerCount()).toBe(0);
    fireEvent.click(screen.getByRole('button', { name: 'Mostra anteprima dal vivo' }));
    unmount();
    expect(spectrumListenerCount()).toBe(0);
  });

  it('says clearly when the engine sends no rows', () => {
    vi.useFakeTimers();
    localStorage.setItem('aurora.wfPreview.open', '1');
    render(<WaterfallPreview />);
    act(() => {
      vi.advanceTimersByTime(1500);
    });
    expect(screen.getByText(/Nessun dato dal motore/)).toBeInTheDocument();
    act(() => {
      emitSpectrum(new Uint8Array(10));
      vi.advanceTimersByTime(1000);
    });
    expect(screen.queryByText(/Nessun dato dal motore/)).toBeNull();
  });
});
