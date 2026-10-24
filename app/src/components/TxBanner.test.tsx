import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { StationStatus } from '../api/schemas';
import { TxBanner } from './TxBanner';

const base: StationStatus = {
  dial_hz: 21_074_000, mode: 'FT8', tx_enabled: false, transmitting: true, decoding: false,
  de_call: 'AA1AA', de_grid: 'FN42', dx_call: 'BB2BB', tx_message: 'BB2BB AA1AA -10', tx_df: 1500,
};

describe('TxBanner', () => {
  it('is red and loud while transmitting and shows what is being sent', () => {
    render(<TxBanner status={base} />);
    const banner = screen.getByRole('alert');
    expect(banner).toHaveTextContent('IN TRASMISSIONE');
    expect(banner).toHaveTextContent('BB2BB AA1AA -10');
    expect(banner).toHaveTextContent('1500 Hz');
    expect(banner).toHaveAttribute('data-state', 'tx');
  });

  it('is green while listening', () => {
    render(<TxBanner status={{ ...base, transmitting: false, decoding: true }} />);
    const bar = screen.getByRole('status');
    expect(bar).toHaveTextContent('IN ASCOLTO');
    expect(bar).toHaveAttribute('data-state', 'rx');
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('says when transmission is armed while listening', () => {
    render(<TxBanner status={{ ...base, transmitting: false, tx_enabled: true }} />);
    expect(screen.getByRole('status')).toHaveTextContent('Tx abilitato');
  });

  it('does not claim Tx is armed when it is not', () => {
    render(<TxBanner status={{ ...base, transmitting: false, tx_enabled: false }} />);
    expect(screen.getByRole('status')).not.toHaveTextContent('Tx abilitato');
  });

  it('is absent without a status', () => {
    render(<TxBanner status={null} />);
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });
});
