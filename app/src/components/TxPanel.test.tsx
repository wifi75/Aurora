import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render as rtlRender, screen } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getControls: vi.fn().mockResolvedValue({}), postControl: vi.fn(), getOutputLevel: vi.fn().mockResolvedValue({ attenuation_db: 0, min_db: 0, max_db: 45 }), putOutputLevel: vi.fn(), postCq: vi.fn().mockResolvedValue(undefined), postFreeText: vi.fn().mockResolvedValue(undefined), postTxFrequency: vi.fn().mockResolvedValue(undefined) }));
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

  it('shows the transmit and receive frequencies', () => {
    render(<TxPanel txAllowed status={status} />);
    expect(screen.getByText('Tx 1500 Hz')).toBeInTheDocument();
    expect(screen.getByText('Rx 1200 Hz')).toBeInTheDocument();
  });
});
