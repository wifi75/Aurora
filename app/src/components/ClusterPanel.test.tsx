import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ClusterSpot } from '../api/client';

vi.mock('../api/client', () => ({
  getClusterSpots: vi.fn(),
  getStationInfo: vi.fn().mockResolvedValue({ rig_control: true }),
  postFrequency: vi.fn().mockResolvedValue({ dial_hz: 14_074_000 }),
  ApiError: class extends Error {},
}));
import { getClusterSpots, postFrequency } from '../api/client';
import { useStation } from '../state/station';
import { ClusterPanel } from './ClusterPanel';

const spot = (id: number, call: string, band: string, mode: string, country: string): ClusterSpot => ({
  id, received_ms: Date.now(), server: 'Prova', spotter: 'W3LPL', freq_khz: 14074, dx_call: call, comment: 'FT8 -10 dB', time_utc: '1234', grid: null, band, mode, country, continent: 'EU',
});

const config = (enabled: boolean, callsign = 'IU3ABC') => ({ enabled, callsign, servers: [], login_commands: [], max_spots: 500, max_age_min: 60 });

function show() {
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <ClusterPanel />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.mocked(postFrequency).mockClear();
  localStorage.clear();
  useStation.setState({ status: { dial_hz: 14_074_000, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: true, de_call: 'IU3ABC', de_grid: 'JN55' } });
});

describe('ClusterPanel', () => {
  it('says it is off and links to the settings', async () => {
    vi.mocked(getClusterSpots).mockResolvedValue({ config: config(false), servers: [], spots: [] });
    show();
    expect(await screen.findByText(/DX Cluster è spento/)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Apri le impostazioni' })).toHaveAttribute('href', '#/impostazioni/cluster');
  });

  it('asks for the callsign when it is missing', async () => {
    vi.mocked(getClusterSpots).mockResolvedValue({ config: config(true, ''), servers: [], spots: [] });
    show();
    expect(await screen.findByText(/Manca il tuo nominativo/)).toBeInTheDocument();
  });

  it('lists the spots with the country and says how many servers are connected', async () => {
    vi.mocked(getClusterSpots).mockResolvedValue({
      config: config(true),
      servers: [{ key: 'a:1', name: 'A', state: 'connected', detail: null, spots: 2, since_ms: 0 }, { key: 'b:1', name: 'B', state: 'error', detail: 'x', spots: 0, since_ms: 0 }],
      spots: [spot(2, 'JA1ABC', '20m', 'FT8', 'Japan'), spot(1, 'K1ABC', '40m', 'CW', 'United States')],
    });
    show();
    expect(await screen.findByText('JA1ABC')).toBeInTheDocument();
    expect(screen.getByText('Japan')).toBeInTheDocument();
    expect(screen.getByText('Collegato a 1 su 2')).toBeInTheDocument();
  });

  it('shows only my band when asked', async () => {
    vi.mocked(getClusterSpots).mockResolvedValue({ config: config(true), servers: [], spots: [spot(2, 'JA1ABC', '20m', 'FT8', 'Japan'), spot(1, 'K1ABC', '40m', 'CW', 'United States')] });
    show();
    await screen.findByText('K1ABC');
    fireEvent.click(screen.getByRole('button', { name: 'Banda' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'La mia banda' }));
    expect(screen.queryByText('K1ABC')).toBeNull();
    expect(screen.getByText('JA1ABC')).toBeInTheDocument();
  });

  it('a double click on a spot moves the radio to its exact frequency', async () => {
    vi.mocked(getClusterSpots).mockResolvedValue({ config: config(true), servers: [], spots: [spot(1, 'K1ABC', '40m', 'CW', 'United States')] });
    show();
    fireEvent.doubleClick(await screen.findByText('K1ABC'));
    await waitFor(() => expect(postFrequency).toHaveBeenCalledWith(14_074_000));
    expect(await screen.findByText('Radio portata su 14.074,0 kHz.')).toBeInTheDocument();
  });

  it('a single click only selects: the radio does not move', async () => {
    vi.mocked(getClusterSpots).mockResolvedValue({ config: config(true), servers: [], spots: [spot(1, 'JA1ABC', '20m', 'FT8', 'Japan')] });
    show();
    fireEvent.click(await screen.findByText('JA1ABC'));
    expect(postFrequency).not.toHaveBeenCalled();
  });
});
