import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render as rtlRender, screen } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getClock: vi.fn() }));
import { getClock } from '../api/client';
import { useStation } from '../state/station';
import { DiagnosisBar } from './DiagnosisBar';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

const clock = (offset_ms: number | null) => ({
  config: { enabled: true, servers: [], interval_secs: 600, warn_ms: 200, auto_correct: false },
  last: { at_ms: 1, server: 'x', offset_ms, delay_ms: 10, servers_ok: 1, servers_tried: 1, error: null, corrected: false },
});
const station = (de_call = 'IU3ABC') => useStation.getState().apply({ type: 'status', dial_hz: 14_074_000, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: false, de_call, de_grid: 'IO91wm' });

beforeEach(() => {
  useStation.getState().reset();
  vi.mocked(getClock).mockReset().mockResolvedValue(clock(10) as never);
});

describe('DiagnosisBar', () => {
  it('shows nothing when everything is fine', async () => {
    station();
    render(<DiagnosisBar />);
    await vi.waitFor(() => expect(getClock).toHaveBeenCalled());
    expect(screen.queryByRole('list', { name: 'Diagnosi della stazione' })).toBeNull();
  });

  it('warns about the clock with a link to fix it', async () => {
    vi.mocked(getClock).mockResolvedValue(clock(900) as never);
    station();
    render(<DiagnosisBar />);
    expect(await screen.findByText(/L’orologio del PC è fuori di 900 ms/)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Sistema' })).toHaveAttribute('href', '#/impostazioni/clock');
  });

  it('warns about a missing callsign and an overdriven input', async () => {
    station('');
    useStation.getState().apply({ type: 'audio_level', peak: 1, rms: 0.7, clipped: 0.2, state: 'overdrive' });
    render(<DiagnosisBar />);
    expect(await screen.findByText(/manca il tuo nominativo/)).toBeInTheDocument();
    expect(screen.getByText(/L’ingresso audio è saturo/)).toBeInTheDocument();
  });
});
