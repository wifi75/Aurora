import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', async (importActual) => ({
  ApiError: (await importActual<typeof import('../api/client')>()).ApiError,
  getControls: vi.fn(),
  postControl: vi.fn().mockResolvedValue(undefined),
  postTxFrequency: vi.fn().mockResolvedValue(undefined),
  postRxFrequency: vi.fn().mockResolvedValue(undefined),
  postReport: vi.fn().mockResolvedValue(undefined),
  getEngineSettings: vi.fn().mockResolvedValue({}),
  postTune: vi.fn().mockResolvedValue(undefined),
  postSpotDxsummit: vi.fn().mockResolvedValue(undefined),
}));
import { getControls, postControl, postReport, postRxFrequency, postTxFrequency } from '../api/client';
import type { StationStatus } from '../api/schemas';
import { TxCommands } from './TxCommands';

const status: StationStatus = {
  dial_hz: 21_074_000, mode: 'FT8', tx_enabled: true, transmitting: true, decoding: false,
  de_call: 'AA1AA', de_grid: 'FN42', dx_call: 'BB2BB', dx_grid: 'KK00', report: '-10',
  tx_df: 1500, rx_df: 1200, tx_message: 'BB2BB AA1AA -10', tr_period: 15,
};

/** Il pannello legge lo stato degli interruttori da Aurora: serve il contesto delle query. */
function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  vi.mocked(postTxFrequency).mockClear();
  vi.mocked(postRxFrequency).mockClear();
  vi.mocked(postReport).mockClear();
  vi.mocked(postControl).mockClear();
  vi.mocked(getControls).mockReset().mockResolvedValue({ autoseq: true, single_qso: false, enable_tx: false, auto_tx: false, hold_tx: false });
});

describe('TxCommands', () => {
  it('with transmission closed it explains it', () => {
    render(<TxCommands txAllowed={false} />);
    expect(screen.getByText(/trasmissione/i)).toBeInTheDocument();
  });

  it('starts the audio frequencies from the ones the engine reports', () => {
    render(<TxCommands txAllowed status={status} />);
    expect(screen.getByLabelText(/^Tx$/)).toHaveValue(1500);
    expect(screen.getByLabelText(/^Rx$/)).toHaveValue(1200);
  });

  it('applies the transmit and receive frequencies on Enter', async () => {
    render(<TxCommands txAllowed={false} />);
    const tx = screen.getByLabelText(/^Tx$/);
    fireEvent.change(tx, { target: { value: '1800' } });
    fireEvent.keyDown(tx, { key: 'Enter' });
    await waitFor(() => expect(postTxFrequency).toHaveBeenCalledWith(1800));
    const rx = screen.getByLabelText(/^Rx$/);
    fireEvent.change(rx, { target: { value: '900' } });
    fireEvent.keyDown(rx, { key: 'Enter' });
    await waitFor(() => expect(postRxFrequency).toHaveBeenCalledWith(900));
  });

  it('copies one frequency onto the other with the arrows', async () => {
    render(<TxCommands txAllowed status={status} />);
    fireEvent.click(await screen.findByRole('button', { name: /Tx=Rx/ }));
    await waitFor(() => expect(postControl).toHaveBeenCalledWith('rx_to_tx', undefined));
    fireEvent.click(screen.getByRole('button', { name: /Rx=Tx/ }));
    await waitFor(() => expect(postControl).toHaveBeenCalledWith('tx_to_rx', undefined));
  });

  it('sends the report on Enter', async () => {
    render(<TxCommands txAllowed status={status} />);
    const report = screen.getByLabelText(/rapporto/i);
    fireEvent.change(report, { target: { value: '-7' } });
    fireEvent.keyDown(report, { key: 'Enter' });
    await waitFor(() => expect(postReport).toHaveBeenCalledWith(-7));
  });

  it('turns Hold Tx Freq on and off', async () => {
    render(<TxCommands txAllowed status={status} />);
    fireEvent.click(await screen.findByRole('button', { name: /hold tx freq/i }));
    await waitFor(() => expect(postControl).toHaveBeenCalledWith('hold_tx', true));
  });
});
