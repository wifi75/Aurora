import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getClusterSpots: vi.fn() }));
import { getClusterSpots, type ClusterSpot } from '../api/client';
import { SpotMarkers } from './SpotMarkers';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

const spot = (id: number, call: string, khz: number): ClusterSpot => ({
  id, received_ms: Date.now() - 30_000, server: 'Prova', spotter: 'W3LPL', freq_khz: khz, dx_call: call, comment: '', time_utc: '1234', grid: null, band: '20m', mode: 'FT8', country: null, continent: null,
});
const config = (enabled: boolean) => ({ enabled, callsign: 'IU3ABC', servers: [], max_spots: 200 }) as never;
const reply = (enabled: boolean, spots: ClusterSpot[]) => ({ config: config(enabled), servers: [], spots });

beforeEach(() => {
  vi.mocked(getClusterSpots).mockReset();
});

describe('SpotMarkers', () => {
  it('draws the spots that fall inside the waterfall and picks the receive frequency on click', async () => {
    vi.mocked(getClusterSpots).mockResolvedValue(reply(true, [spot(1, 'JA1ABC', 14074.5), spot(2, 'K1ABC', 14090)]));
    const onPick = vi.fn();
    render(<SpotMarkers fMinHz={0} fMaxHz={3000} dialHz={14_074_000} onPick={onPick} />);
    const button = await screen.findByRole('button', { name: /JA1ABC/ });
    expect(screen.queryByRole('button', { name: /K1ABC/ })).toBeNull();
    expect(button).toHaveTextContent('JA1ABC');
    fireEvent.click(button);
    expect(onPick).toHaveBeenCalledWith(500);
  });

  it('shows nothing when the cluster is off or the radio frequency is not known', async () => {
    vi.mocked(getClusterSpots).mockResolvedValue(reply(false, [spot(1, 'JA1ABC', 14074.5)]));
    const { container, unmount } = render(<SpotMarkers fMinHz={0} fMaxHz={3000} dialHz={14_074_000} onPick={() => {}} />);
    await vi.waitFor(() => expect(getClusterSpots).toHaveBeenCalled());
    expect(container.querySelector('.wf-spots')).toBeNull();
    unmount();
    vi.mocked(getClusterSpots).mockResolvedValue(reply(true, [spot(1, 'JA1ABC', 14074.5)]));
    const second = render(<SpotMarkers fMinHz={0} fMaxHz={3000} dialHz={undefined} onPick={() => {}} />);
    await vi.waitFor(() => expect(getClusterSpots).toHaveBeenCalledTimes(2));
    expect(second.container.querySelector('.wf-spots')).toBeNull();
  });
});
