import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({
  getControls: vi.fn(),
  postControl: vi.fn().mockResolvedValue(undefined),
  postCq: vi.fn().mockResolvedValue(undefined),
  postFreeText: vi.fn().mockResolvedValue(undefined),
  postTxFrequency: vi.fn().mockResolvedValue(undefined),
  postRxFrequency: vi.fn().mockResolvedValue(undefined),
}));
import { getControls, postControl, postCq, postFreeText, postRxFrequency, postTxFrequency } from '../api/client';
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
  vi.mocked(postCq).mockClear();
  vi.mocked(postFreeText).mockClear();
  vi.mocked(postTxFrequency).mockClear();
  vi.mocked(postRxFrequency).mockClear();
  vi.mocked(postControl).mockClear();
  vi.mocked(getControls).mockReset().mockResolvedValue({ autoseq: true, single_qso: false, enable_tx: false, auto_tx: false });
});

describe('TxCommands', () => {
  it('fills the free message from a macro with the DX call and report', async () => {
    render(<TxCommands txAllowed status={status} />);
    fireEvent.click(screen.getByRole('button', { name: '@ R#' }));
    expect(screen.getByLabelText(/messaggio libero/i)).toHaveValue('BB2BB R-10');
    fireEvent.click(screen.getByRole('button', { name: /prepara/i }));
    await waitFor(() => expect(postFreeText).toHaveBeenCalledWith('BB2BB R-10', false));
  });

  it('with transmission closed it explains and disables what would transmit', () => {
    render(<TxCommands txAllowed={false} />);
    expect(screen.getByText(/trasmissione chiusa/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^CQ/ })).toBeDisabled();
    expect(screen.getByRole('button', { name: /invia/i })).toBeDisabled();
  });

  it('calls CQ, with DX when chosen', async () => {
    render(<TxCommands txAllowed />);
    fireEvent.click(screen.getByRole('button', { name: /^CQ$/ }));
    await waitFor(() => expect(postCq).toHaveBeenCalledWith(''));
    fireEvent.click(screen.getByLabelText('DX'));
    fireEvent.click(screen.getByRole('button', { name: /^CQ DX$/ }));
    await waitFor(() => expect(postCq).toHaveBeenLastCalledWith('DX'));
  });

  it('sends a free text in uppercase, limited to 13 characters', async () => {
    render(<TxCommands txAllowed />);
    const box = screen.getByLabelText(/messaggio libero/i);
    fireEvent.change(box, { target: { value: 'tnx 73 gl dx e altro' } });
    expect(box).toHaveValue('TNX 73 GL DX ');
    fireEvent.click(screen.getByRole('button', { name: /invia/i }));
    await waitFor(() => expect(postFreeText).toHaveBeenCalledWith('TNX 73 GL DX', true));
  });

  it('prepares a message without transmitting, even with transmission closed', async () => {
    render(<TxCommands txAllowed={false} />);
    fireEvent.change(screen.getByLabelText(/messaggio libero/i), { target: { value: '73' } });
    fireEvent.click(screen.getByRole('button', { name: /prepara/i }));
    await waitFor(() => expect(postFreeText).toHaveBeenCalledWith('73', false));
  });




  it('presses the simple buttons without a state', async () => {
    render(<TxCommands txAllowed status={status} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Canc DX' }));
    await waitFor(() => expect(postControl).toHaveBeenCalledWith('clear_dx', undefined));
    fireEvent.click(screen.getByRole('button', { name: 'Gen.Msgs' }));
    await waitFor(() => expect(postControl).toHaveBeenCalledWith('gen_msgs', undefined));
  });

  it('shows the DX station and its locator', () => {
    render(<TxCommands txAllowed status={status} />);
    expect(screen.getByText('BB2BB')).toBeInTheDocument();
    expect(screen.getByText('KK00')).toBeInTheDocument();
  });

  it('starts the audio frequencies from the ones the engine reports', () => {
    render(<TxCommands txAllowed status={status} />);
    expect(screen.getByLabelText(/^Tx \(Hz\)/)).toHaveValue(1500);
    expect(screen.getByLabelText(/^Rx \(Hz\)/)).toHaveValue(1200);
  });

  it('applies the transmit and receive frequencies on Enter', async () => {
    render(<TxCommands txAllowed={false} />);
    const tx = screen.getByLabelText(/^Tx \(Hz\)/);
    fireEvent.change(tx, { target: { value: '1800' } });
    fireEvent.keyDown(tx, { key: 'Enter' });
    await waitFor(() => expect(postTxFrequency).toHaveBeenCalledWith(1800));
    const rx = screen.getByLabelText(/^Rx \(Hz\)/);
    fireEvent.change(rx, { target: { value: '900' } });
    fireEvent.keyDown(rx, { key: 'Enter' });
    await waitFor(() => expect(postRxFrequency).toHaveBeenCalledWith(900));
  });

  it('copies one frequency onto the other with the arrows', async () => {
    render(<TxCommands txAllowed status={status} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Rx → Tx' }));
    await waitFor(() => expect(postControl).toHaveBeenCalledWith('rx_to_tx', undefined));
    fireEvent.click(screen.getByRole('button', { name: 'Tx → Rx' }));
    await waitFor(() => expect(postControl).toHaveBeenCalledWith('tx_to_rx', undefined));
  });

});
