import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getOutputLevel: vi.fn(), putOutputLevel: vi.fn().mockResolvedValue(undefined), ApiError: class extends Error {} }));
import { getOutputLevel, putOutputLevel } from '../api/client';
import { OutputLevel } from './OutputLevel';

function show(txAllowed = true) {
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <OutputLevel txAllowed={txAllowed} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.mocked(getOutputLevel).mockReset().mockResolvedValue({ attenuation_db: 22.5, min_db: 0, max_db: 45 });
  vi.mocked(putOutputLevel).mockClear();
});

describe('OutputLevel', () => {
  it('shows the current level and the attenuation', async () => {
    show();
    const slider = await screen.findByRole('slider', { name: /livello audio di uscita/i });
    await waitFor(() => expect(slider).toHaveAttribute('aria-valuenow', '50'));
    const value = screen.getByText('50%');
    expect(value).toHaveAttribute('title', expect.stringMatching(/50 % · attenuazione 22.5 dB/));
  });

  it('sends the new level only when released, and up means more level', async () => {
    show();
    const slider = await screen.findByRole('slider', { name: /livello audio di uscita/i });
    await waitFor(() => expect(slider).toHaveAttribute('aria-valuenow', '50'));
    // binario alto 100 px a partire da y=0: il punto y=20 e' in alto, quindi l'80%
    slider.getBoundingClientRect = () => ({ top: 0, height: 100, bottom: 100, left: 0, right: 10, width: 10, x: 0, y: 0, toJSON: () => ({}) });
    fireEvent.pointerDown(slider, { clientY: 20, pointerId: 1 });
    expect(slider).toHaveAttribute('aria-valuenow', '80');
    fireEvent.pointerMove(slider, { clientY: 30, pointerId: 1 });
    expect(slider).toHaveAttribute('aria-valuenow', '70');
    expect(putOutputLevel).not.toHaveBeenCalled();
    fireEvent.pointerUp(slider, { pointerId: 1 });
    await waitFor(() => expect(putOutputLevel).toHaveBeenCalledTimes(1));
    expect(putOutputLevel).toHaveBeenCalledWith(13.5);
  });

  it('the top of the track is 100% (no attenuation) and the bottom is 0%', async () => {
    show();
    const slider = await screen.findByRole('slider', { name: /livello audio di uscita/i });
    await waitFor(() => expect(slider).toHaveAttribute('aria-valuenow', '50'));
    slider.getBoundingClientRect = () => ({ top: 0, height: 100, bottom: 100, left: 0, right: 10, width: 10, x: 0, y: 0, toJSON: () => ({}) });
    fireEvent.pointerDown(slider, { clientY: 0, pointerId: 1 });
    fireEvent.pointerUp(slider, { pointerId: 1 });
    await waitFor(() => expect(putOutputLevel).toHaveBeenLastCalledWith(0));
    fireEvent.pointerDown(slider, { clientY: 100, pointerId: 1 });
    fireEvent.pointerUp(slider, { pointerId: 1 });
    await waitFor(() => expect(putOutputLevel).toHaveBeenLastCalledWith(45));
  });

  it('works from the keyboard: arrow up raises the level and the value is sent on key up', async () => {
    show();
    const slider = await screen.findByRole('slider', { name: /livello audio di uscita/i });
    await waitFor(() => expect(slider).toHaveAttribute('aria-valuenow', '50'));
    fireEvent.keyDown(slider, { key: 'ArrowUp' });
    expect(slider).toHaveAttribute('aria-valuenow', '51');
    fireEvent.keyUp(slider, { key: 'ArrowUp' });
    await waitFor(() => expect(putOutputLevel).toHaveBeenCalledWith(22.1));
  });

  it('is locked when transmission is closed and explains why in its tooltip', async () => {
    show(false);
    const slider = await screen.findByRole('slider', { name: /livello audio di uscita/i });
    expect(slider).toHaveAttribute('aria-disabled', 'true');
    fireEvent.keyDown(slider, { key: 'ArrowUp' });
    expect(slider).toHaveAttribute('aria-valuenow', '50');
    expect(screen.getByTitle(/ALC/)).toBeInTheDocument();
  });

  it('says so when the level cannot be read', async () => {
    vi.mocked(getOutputLevel).mockRejectedValue(new Error('503'));
    show();
    expect(await screen.findByTitle(/non disponibile/i)).toBeInTheDocument();
  });
});
