import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getOutputLevel: vi.fn().mockResolvedValue({ attenuation_db: 0, min_db: 0, max_db: 45 }), putOutputLevel: vi.fn(), postCq: vi.fn().mockResolvedValue(undefined), postFreeText: vi.fn().mockResolvedValue(undefined), postTxFrequency: vi.fn().mockResolvedValue(undefined) }));
import { postCq, postFreeText, postTxFrequency } from '../api/client';
import type { StationStatus } from '../api/schemas';
import { TxPanel } from './TxPanel';

const status: StationStatus = {
  dial_hz: 21_074_000, mode: 'FT8', tx_enabled: true, transmitting: true, decoding: false,
  de_call: 'AA1AA', de_grid: 'FN42', dx_call: 'BB2BB', dx_grid: 'KK00', report: '-10',
  tx_df: 1500, rx_df: 1200, tx_message: 'BB2BB AA1AA -10', tr_period: 15,
};

/** Il pannello contiene il livello di uscita, che legge da Aurora: serve il contesto delle query. */
function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  vi.mocked(postCq).mockClear();
  vi.mocked(postFreeText).mockClear();
  vi.mocked(postTxFrequency).mockClear();
});

describe('TxPanel', () => {
  it('shows the DX station and the six messages, lighting the one being sent', () => {
    render(<TxPanel txAllowed status={status} />);
    expect(screen.getByText('BB2BB')).toBeInTheDocument();
    expect(screen.getByText('BB2BB AA1AA FN42')).toBeInTheDocument();
    expect(screen.getByText('CQ AA1AA FN42')).toBeInTheDocument();
    expect(screen.getByText('BB2BB AA1AA -10').closest('li')).toHaveAttribute('aria-current', 'true');
    expect(screen.getByText('BB2BB AA1AA 73').closest('li')).not.toHaveAttribute('aria-current');
  });

  it('lights the phase of the QSO that comes next when nothing matches what JTDX is sending', () => {
    const quiet = { ...status, transmitting: false, tx_message: null };
    render(<TxPanel txAllowed status={quiet} decodes={[{ id: 1, time_ms: 1, snr: -5, delta_time: 0, freq_hz: 1500, text: 'AA1AA BB2BB -12', tag: null }]} />);
    const next = screen.getByText('BB2BB AA1AA R-10').closest('li');
    expect(next).toHaveAttribute('data-next', 'true');
    expect(next).not.toHaveAttribute('aria-current');
    expect(screen.getByText('BB2BB AA1AA FN42').closest('li')).not.toHaveAttribute('data-next');
  });

  it('prefers the exact message JTDX is sending over the predicted phase', () => {
    render(<TxPanel txAllowed status={status} decodes={[]} />);
    expect(screen.getByText('BB2BB AA1AA -10').closest('li')).toHaveAttribute('aria-current', 'true');
    expect(document.querySelector('[data-next]')).toBeNull();
  });

  it('fills the free message from a macro with the DX call and report', async () => {
    render(<TxPanel txAllowed status={status} />);
    fireEvent.click(screen.getByRole('button', { name: '@ R#' }));
    expect(screen.getByLabelText(/messaggio libero/i)).toHaveValue('BB2BB R-10');
    fireEvent.click(screen.getByRole('button', { name: /prepara/i }));
    await waitFor(() => expect(postFreeText).toHaveBeenCalledWith('BB2BB R-10', false));
  });

  it('shows the transmit and receive frequencies', () => {
    render(<TxPanel txAllowed status={status} />);
    expect(screen.getByText('Tx 1500 Hz')).toBeInTheDocument();
    expect(screen.getByText('Rx 1200 Hz')).toBeInTheDocument();
  });

  it('with transmission closed it explains and disables what would transmit', () => {
    render(<TxPanel txAllowed={false} />);
    expect(screen.getByText(/trasmissione chiusa/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^CQ/ })).toBeDisabled();
    expect(screen.getByRole('button', { name: /invia/i })).toBeDisabled();
  });

  it('calls CQ, with DX when chosen', async () => {
    render(<TxPanel txAllowed />);
    fireEvent.click(screen.getByRole('button', { name: /^CQ$/ }));
    await waitFor(() => expect(postCq).toHaveBeenCalledWith(''));
    fireEvent.click(screen.getByLabelText('DX'));
    fireEvent.click(screen.getByRole('button', { name: /^CQ DX$/ }));
    await waitFor(() => expect(postCq).toHaveBeenLastCalledWith('DX'));
  });

  it('sends a free text in uppercase, limited to 13 characters', async () => {
    render(<TxPanel txAllowed />);
    const box = screen.getByLabelText(/messaggio libero/i);
    fireEvent.change(box, { target: { value: 'tnx 73 gl dx e altro' } });
    expect(box).toHaveValue('TNX 73 GL DX ');
    fireEvent.click(screen.getByRole('button', { name: /invia/i }));
    await waitFor(() => expect(postFreeText).toHaveBeenCalledWith('TNX 73 GL DX', true));
  });

  it('prepares a message without transmitting, even with transmission closed', async () => {
    render(<TxPanel txAllowed={false} />);
    fireEvent.change(screen.getByLabelText(/messaggio libero/i), { target: { value: '73' } });
    fireEvent.click(screen.getByRole('button', { name: /prepara/i }));
    await waitFor(() => expect(postFreeText).toHaveBeenCalledWith('73', false));
  });

  it('sets the transmit frequency within the audio band', async () => {
    render(<TxPanel txAllowed={false} />);
    fireEvent.change(screen.getByLabelText(/frequenza di trasmissione/i), { target: { value: '1800' } });
    fireEvent.click(screen.getByRole('button', { name: /imposta/i }));
    await waitFor(() => expect(postTxFrequency).toHaveBeenCalledWith(1800));
  });
});
