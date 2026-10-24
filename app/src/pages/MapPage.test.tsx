import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({
  getWorkedMap: vi.fn().mockResolvedValue({
    squares: [
      { grid: 'JN45', count: 3, bands: ['20m', '15m'] },
      { grid: 'FN31', count: 1, bands: ['40m'] },
    ],
  }),
}));
import { getWorkedMap } from '../api/client';
import { useStation } from '../state/station';
import { MapPage } from './MapPage';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  useStation.getState().reset();
  vi.mocked(getWorkedMap).mockClear();
});

describe('MapPage', () => {
  it('draws the worked squares of the log and counts them', async () => {
    render(<MapPage />);
    expect(await screen.findByText(/2 griglie lavorate · 4 QSO/)).toBeInTheDocument();
    expect(document.querySelectorAll('.worldmap-worked rect')).toHaveLength(2);
  });

  it('filters the worked squares by band', async () => {
    render(<MapPage />);
    await screen.findByText(/2 griglie lavorate/);
    fireEvent.change(screen.getByRole('combobox'), { target: { value: '40m' } });
    expect(document.querySelectorAll('.worldmap-worked rect')).toHaveLength(1);
    fireEvent.change(screen.getByRole('combobox'), { target: { value: '15m' } });
    expect(document.querySelectorAll('.worldmap-worked rect')).toHaveLength(1);
  });

  it('shows the stations received now and their details on a click', async () => {
    useStation.getState().apply({ type: 'decode', id: 1, time_ms: 1, snr: -12, delta_time: 0, freq_hz: 1500, text: 'CQ K1ABC FN42', tag: null, country: 'United States' });
    useStation.getState().apply({ type: 'decode', id: 2, time_ms: 2, snr: -5, delta_time: 0, freq_hz: 1500, text: 'K1ABC G0XYZ RR73', tag: null });
    render(<MapPage />);
    expect(await screen.findByText(/1 stazioni con locatore/)).toBeInTheDocument();
    const dot = document.querySelector('.worldmap-station')!;
    fireEvent.click(dot);
    expect(document.querySelector('.map-picked')?.textContent).toContain('K1ABC · FN42 · -12 dB · United States');
  });

  it('can hide each layer and says so when the log is not connected', async () => {
    vi.mocked(getWorkedMap).mockRejectedValueOnce(new Error('404'));
    render(<MapPage />);
    expect(await screen.findByText(/Il log di Log4OM2 non è collegato/)).toBeInTheDocument();
    expect(document.querySelectorAll('.worldmap-worked rect')).toHaveLength(0);
  });

  it('hides the squares and the stations with their check boxes', async () => {
    useStation.getState().apply({ type: 'decode', id: 1, time_ms: 1, snr: -12, delta_time: 0, freq_hz: 1500, text: 'CQ K1ABC FN42', tag: null });
    render(<MapPage />);
    await waitFor(() => expect(document.querySelectorAll('.worldmap-worked rect')).toHaveLength(2));
    fireEvent.click(screen.getByLabelText('Griglie lavorate'));
    expect(document.querySelectorAll('.worldmap-worked rect')).toHaveLength(0);
    fireEvent.click(screen.getByLabelText('Stazioni ricevute ora'));
    expect(document.querySelectorAll('.worldmap-station')).toHaveLength(0);
  });
});
