import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({
  getWishlist: vi.fn(),
  postFrequency: vi.fn().mockResolvedValue({ dial_hz: 14_195_000 }),
}));
import { getWishlist, postFrequency } from '../api/client';
import { EMPTY_WISHLIST } from '../lib/wishlist';
import { useStation } from '../state/station';
import { useWishlist } from '../state/wishlist';
import { WantedBar } from './WantedBar';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

const alert = { type: 'wanted' as const, call: '3Y0J', reason: 'call', source: 'cluster', text: 'up 5', freq_hz: 14_195_000, band: '20m', snr: null, country: 'Bouvet', grid: null, time_ms: 1 };

beforeEach(() => {
  useStation.getState().reset();
  useWishlist.getState().set(EMPTY_WISHLIST);
  vi.mocked(getWishlist).mockReset().mockResolvedValue({ config: { ...EMPTY_WISHLIST, enabled: true, calls: ['3Y0J'] }, recent: [] });
  vi.mocked(postFrequency).mockClear();
});

describe('WantedBar', () => {
  it('shows nothing until a wanted station appears, then the bar with the details', async () => {
    render(<WantedBar />);
    expect(screen.queryByRole('alert')).toBeNull();
    useStation.getState().apply(alert);
    const bar = await screen.findByRole('alert');
    expect(bar).toHaveTextContent('3Y0J desiderata (nominativo)');
    expect(bar).toHaveTextContent('20m');
    expect(bar).toHaveTextContent('14.195,0 kHz');
    expect(bar).toHaveTextContent('Bouvet');
  });

  it('closes, and for a cluster spot goes to the frequency', async () => {
    render(<WantedBar />);
    useStation.getState().apply(alert);
    fireEvent.click(await screen.findByRole('button', { name: 'Vai alla frequenza' }));
    await waitFor(() => expect(postFrequency).toHaveBeenCalledWith(14_195_000));
    fireEvent.click(screen.getByRole('button', { name: 'Chiudi' }));
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('has no tune button for an alert from a decoded message', async () => {
    render(<WantedBar />);
    useStation.getState().apply({ ...alert, source: 'decode', text: 'CQ 3Y0J JD15', freq_hz: null });
    await screen.findByRole('alert');
    expect(screen.queryByRole('button', { name: 'Vai alla frequenza' })).toBeNull();
  });

  it('puts the list in use in the shared store, so the lists can highlight', async () => {
    render(<WantedBar />);
    await waitFor(() => expect(useWishlist.getState().list.calls).toEqual(['3Y0J']));
  });
});
