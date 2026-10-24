import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getOnline: vi.fn(), getPsk: vi.fn(), getSolar: vi.fn(), refreshOnline: vi.fn().mockResolvedValue({ asked: true }) }));
import { getOnline, getPsk, getSolar, refreshOnline, type PskState, type SolarState } from '../api/client';
import { useStation } from '../state/station';
import { PropagationPage } from './PropagationPage';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

const solar: SolarState = {
  enabled: true,
  solar: {
    fetched_ms: 1,
    error: null,
    data: {
      updated: '24 Oct 2026 1758 GMT', sfi: 101, a_index: 11, k_index: 1, xray: 'B4.3', sunspots: 25, aurora: 2, solar_wind: 408.7, magnetic_field: -1.1, geomag: 'VR QUIET', signal_noise: 'S0-S1', muf: 'NoRpt',
      bands: [
        { name: '80m-40m', time: 'day', condition: 'Fair' },
        { name: '30m-20m', time: 'day', condition: 'Good' },
        { name: '17m-15m', time: 'night', condition: 'Poor' },
      ],
      vhf: [],
    },
  },
};
const psk = (reports: PskState['psk']['reports'], error: string | null = null): PskState => ({ enabled: true, psk: { fetched_ms: reports.length > 0 || error === null ? 1 : 0, callsign: 'IU3ABC', hours: 3, reports, error } });

beforeEach(() => {
  useStation.getState().reset();
  vi.mocked(getOnline).mockReset().mockResolvedValue({ psk: true, solar: true });
  vi.mocked(getSolar).mockReset().mockResolvedValue(solar);
  vi.mocked(getPsk).mockReset().mockResolvedValue(psk([{ receiver: 'JA1ABC', locator: 'PM95', snr: -12, freq_hz: 14_075_500, mode: 'FT8', time_s: 3_600, country: 'Japan' }]));
  vi.mocked(refreshOnline).mockClear();
});

describe('PropagationPage', () => {
  it('points to the settings when both services are off', async () => {
    vi.mocked(getOnline).mockResolvedValue({ psk: false, solar: false });
    render(<PropagationPage />);
    expect(await screen.findByText(/I servizi online sono spenti/)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Apri le impostazioni' })).toHaveAttribute('href', '#/impostazioni/online');
    expect(getSolar).not.toHaveBeenCalled();
    expect(getPsk).not.toHaveBeenCalled();
  });

  it('shows the indices and the conditions by band group', async () => {
    render(<PropagationPage />);
    expect(await screen.findByText('101')).toBeInTheDocument();
    expect(screen.getByText('VR QUIET')).toBeInTheDocument();
    expect(screen.getByText('Discreta')).toBeInTheDocument();
    expect(screen.getByText('Buona')).toBeInTheDocument();
    expect(screen.getByText('Scarsa')).toBeInTheDocument();
    expect(screen.getByText(/Dati di N0NBH/)).toBeInTheDocument();
  });

  it('lists who received me, with the distance from my grid', async () => {
    useStation.getState().apply({ type: 'status', dial_hz: 14_074_000, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: false, de_call: 'IU3ABC', de_grid: 'IO91wm' });
    render(<PropagationPage />);
    expect(await screen.findByText('JA1ABC')).toBeInTheDocument();
    expect(screen.getByText(/1 stazioni ti hanno ricevuto nelle ultime 3 ore · la più lontana: JA1ABC a \d+ km/)).toBeInTheDocument();
    expect(screen.getByText('Japan')).toBeInTheDocument();
    expect(screen.getByText('20m')).toBeInTheDocument();
  });

  it('says when nobody received me and when a service failed', async () => {
    vi.mocked(getPsk).mockResolvedValue(psk([]));
    vi.mocked(getSolar).mockResolvedValue({ enabled: true, solar: { fetched_ms: 0, data: null, error: 'il servizio ha risposto 503' } });
    render(<PropagationPage />);
    expect(await screen.findByText(/Nessuna stazione ti ha ricevuto nelle ultime 3 ore/)).toBeInTheDocument();
    expect(screen.getByText(/Il servizio non ha risposto: il servizio ha risposto 503/)).toBeInTheDocument();
  });

  it('asks the core to refresh now', async () => {
    render(<PropagationPage />);
    fireEvent.click(await screen.findByRole('button', { name: 'Aggiorna ora' }));
    await waitFor(() => expect(refreshOnline).toHaveBeenCalledTimes(1));
  });

  it('shows only the service that is on', async () => {
    vi.mocked(getOnline).mockResolvedValue({ psk: false, solar: true });
    render(<PropagationPage />);
    expect(await screen.findByText('101')).toBeInTheDocument();
    expect(screen.queryByText('Chi mi ha ricevuto')).toBeNull();
    expect(getPsk).not.toHaveBeenCalled();
  });
});
