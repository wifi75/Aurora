import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../api/client')>()),
  getClusterSpots: vi.fn(async () => ({
    config: { enabled: true, callsign: 'IU3ABC', servers: [], login_commands: [], max_spots: 500, max_age_min: 60 },
    servers: [],
    spots: [{ id: 1, received_ms: Date.now(), server: 'Prova', spotter: 'EA5JLB', freq_khz: 14074, dx_call: 'FR5FK', comment: '', time_utc: '1857', grid: null, band: '20m', mode: 'FT8', country: 'Reunion Island', continent: 'AF' }],
  })),
  getLog4om: vi.fn(async () => ({ total: 1, qsos: [{ date: '2026-11-04 22:17', call: 'AP5ARS', band: '40m', mode: 'FT8', rst_sent: '-08', rst_rcvd: '-12', country: 'Pakistan', grid: '' }] })),
  getControls: vi.fn(async () => ({})),
  getStationInfo: vi.fn(async () => ({})),
}));
import { PhoneLive } from './PhoneLive';

describe('PhoneLive', () => {
  it('shows the country after the callsign in the cluster and in the log', async () => {
    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <PhoneLive waterfall={null} decodes={[]} onReply={() => {}} onQueue={() => {}} pickBand={() => {}} pickMode={() => {}} rigControl={false} txAllowed={false} />
      </QueryClientProvider>,
    );
    expect(await screen.findByText('Reunion Island')).toBeInTheDocument();
    expect(screen.getByText('FR5FK')).toBeInTheDocument();
    expect(await screen.findByText('Pakistan')).toBeInTheDocument();
    expect(screen.getByText('AP5ARS')).toBeInTheDocument();
  });
});
