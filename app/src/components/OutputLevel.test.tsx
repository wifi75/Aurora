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
    await waitFor(() => expect(slider).toHaveValue('50'));
    const value = screen.getByText('50%');
    expect(value).toHaveAttribute('title', expect.stringMatching(/50 % · attenuazione 22.5 dB/));
  });

  it('sends the new level only when the slider is released, not on every step', async () => {
    show();
    const slider = await screen.findByRole('slider', { name: /livello audio di uscita/i });
    await waitFor(() => expect(slider).toHaveValue('50'));
    fireEvent.change(slider, { target: { value: '80' } });
    fireEvent.change(slider, { target: { value: '70' } });
    expect(putOutputLevel).not.toHaveBeenCalled();
    fireEvent.pointerUp(slider);
    await waitFor(() => expect(putOutputLevel).toHaveBeenCalledTimes(1));
    expect(putOutputLevel).toHaveBeenCalledWith(13.5);
  });

  it('is locked when transmission is closed and explains why in its tooltip', async () => {
    show(false);
    const slider = await screen.findByRole('slider', { name: /livello audio di uscita/i });
    expect(slider).toBeDisabled();
    expect(screen.getByTitle(/ALC/)).toBeInTheDocument();
  });

  it('says so when the level cannot be read', async () => {
    vi.mocked(getOutputLevel).mockRejectedValue(new Error('503'));
    show();
    expect(await screen.findByText(/non disponibile/i)).toBeInTheDocument();
  });
});
