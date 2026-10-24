import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ ApiError: class ApiError extends Error {}, getControls: vi.fn().mockResolvedValue({}), postControl: vi.fn(), getOutputLevel: vi.fn().mockResolvedValue({ attenuation_db: 0, min_db: 0, max_db: 45 }), putOutputLevel: vi.fn(), postCq: vi.fn().mockResolvedValue(undefined), postFreeText: vi.fn().mockResolvedValue(undefined), postTxFrequency: vi.fn().mockResolvedValue(undefined), postTxMessage: vi.fn().mockResolvedValue(undefined) }));
import { getControls, postCq, postFreeText, postTxFrequency, postTxMessage } from '../api/client';
import { useEngineTx } from '../state/engineTx';
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
  vi.mocked(postTxMessage).mockReset().mockResolvedValue(undefined);
  vi.mocked(getControls).mockResolvedValue({});
  useEngineTx.getState().set(null);
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

  it('lights the phase of the QSO that comes next when nothing matches what the engine is sending', () => {
    const quiet = { ...status, transmitting: false, tx_message: null };
    render(<TxPanel txAllowed status={quiet} decodes={[{ id: 1, time_ms: 1, snr: -5, delta_time: 0, freq_hz: 1500, text: 'AA1AA BB2BB -12', tag: null }]} />);
    const next = screen.getByText('BB2BB AA1AA R-10').closest('li');
    expect(next).toHaveAttribute('data-next', 'true');
    expect(next).not.toHaveAttribute('aria-current');
    expect(screen.getByText('BB2BB AA1AA FN42').closest('li')).not.toHaveAttribute('data-next');
  });

  it('with Skip Tx1 on, answering a CQ starts from Tx2 and the first line is shown as skipped', async () => {
    vi.mocked(getControls).mockResolvedValue({ skip_tx1: true });
    const quiet = { ...status, transmitting: false, tx_message: null };
    render(<TxPanel txAllowed status={quiet} decodes={[]} />);
    await waitFor(() => expect(screen.getByText('BB2BB AA1AA FN42').closest('li')).toHaveAttribute('data-skipped', 'true'));
    expect(screen.getByText('BB2BB AA1AA -10').closest('li')).toHaveAttribute('data-next', 'true');
    expect(screen.getByText('BB2BB AA1AA FN42').closest('li')).not.toHaveAttribute('data-next');
  });

  it('uses the messages and the selection read from the engine window when it already has the station', () => {
    useEngineTx.getState().set({
      messages: ['BB2BB AA1AA FN42', 'BB2BB AA1AA -07', 'BB2BB AA1AA R-07', 'BB2BB AA1AA RR73', 'BB2BB AA1AA 73', 'CQ DX AA1AA FN42'],
      selected: 2,
    });
    const quiet = { ...status, transmitting: false, tx_message: null };
    render(<TxPanel txAllowed status={quiet} decodes={[]} />);
    expect(screen.getByText('BB2BB AA1AA -07').closest('li')).toHaveAttribute('data-next', 'true');
    expect(screen.getByText('CQ DX AA1AA FN42')).toBeInTheDocument();
    expect(screen.queryByText('BB2BB AA1AA -10')).toBeNull();
  });

  it('a click on a message selects it by hand: it lights at once and the engine is told', async () => {
    useEngineTx.getState().set({
      messages: ['BB2BB AA1AA FN42', 'BB2BB AA1AA -07', 'BB2BB AA1AA R-07', 'BB2BB AA1AA RR73', 'BB2BB AA1AA 73', 'CQ DX AA1AA FN42'],
      selected: 2,
    });
    const quiet = { ...status, transmitting: false, tx_message: null };
    render(<TxPanel txAllowed status={quiet} decodes={[]} />);
    fireEvent.click(screen.getByText('BB2BB AA1AA R-07'));
    await waitFor(() => expect(postTxMessage).toHaveBeenCalledWith(3));
    expect(screen.getByText('BB2BB AA1AA R-07').closest('li')).toHaveAttribute('data-next', 'true');
    expect(screen.getByText('BB2BB AA1AA -07').closest('li')).not.toHaveAttribute('data-next');
  });

  it('keeps the old selection and says why when the engine refuses the choice', async () => {
    vi.mocked(postTxMessage).mockRejectedValue(new Error('x'));
    useEngineTx.getState().set({
      messages: ['BB2BB AA1AA FN42', 'BB2BB AA1AA -07', 'BB2BB AA1AA R-07', 'BB2BB AA1AA RR73', 'BB2BB AA1AA 73', 'CQ DX AA1AA FN42'],
      selected: 2,
    });
    const quiet = { ...status, transmitting: false, tx_message: null };
    render(<TxPanel txAllowed status={quiet} decodes={[]} />);
    fireEvent.click(screen.getByText('BB2BB AA1AA RR73'));
    await waitFor(() => expect(screen.getByRole('status')).toBeInTheDocument());
    expect(screen.getByText('BB2BB AA1AA -07').closest('li')).toHaveAttribute('data-next', 'true');
    expect(screen.getByText('BB2BB AA1AA RR73').closest('li')).not.toHaveAttribute('data-next');
  });

  it('lights the selected message strongly while transmitting and only with a border while waiting', () => {
    useEngineTx.getState().set({
      messages: ['BB2BB AA1AA FN42', 'BB2BB AA1AA -07', 'BB2BB AA1AA R-07', 'BB2BB AA1AA RR73', 'BB2BB AA1AA 73', 'CQ DX AA1AA FN42'],
      selected: 4,
    });
    const { unmount } = render(<TxPanel txAllowed status={{ ...status, transmitting: false, tx_message: null }} decodes={[]} />);
    expect(screen.getByText('BB2BB AA1AA RR73').closest('li')).toHaveAttribute('data-next', 'true');
    unmount();
    render(<TxPanel txAllowed status={{ ...status, transmitting: true, tx_message: 'BB2BB AA1AA RR73' }} decodes={[]} />);
    expect(screen.getByText('BB2BB AA1AA RR73').closest('li')).toHaveAttribute('aria-current', 'true');
  });

  it('does nothing when an empty message is clicked', () => {
    const quiet = { ...status, transmitting: false, tx_message: null, dx_call: null };
    render(<TxPanel txAllowed status={quiet} decodes={[]} />);
    const rows = document.querySelectorAll('.tx-list li');
    fireEvent.click(rows[0]!);
    expect(postTxMessage).not.toHaveBeenCalled();
  });

  it('prefers the exact message the engine is sending over the predicted phase', () => {
    render(<TxPanel txAllowed status={status} decodes={[]} />);
    expect(screen.getByText('BB2BB AA1AA -10').closest('li')).toHaveAttribute('aria-current', 'true');
    expect(document.querySelector('[data-next]')).toBeNull();
  });

  it('shows the transmit and receive frequencies', () => {
    render(<TxPanel txAllowed status={status} />);
    expect(screen.getByText('Tx 1500 Hz')).toBeInTheDocument();
    expect(screen.getByText('Rx 1200 Hz')).toBeInTheDocument();
  });
});
