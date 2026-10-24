import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getControls: vi.fn(), postControl: vi.fn().mockResolvedValue(undefined) }));
import { getControls, postControl } from '../api/client';
import { useEngineTx } from '../state/engineTx';
import { useStation } from '../state/station';
import { QsoSwitches } from './QsoSwitches';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  vi.mocked(postControl).mockClear();
  vi.mocked(getControls).mockReset().mockResolvedValue({ autoseq: true, single_qso: false, enable_tx: false, auto_tx: false });
});

describe('QsoSwitches', () => {
  it('shows the state of the engine switches and sets the wanted state when one is pressed', async () => {
    render(<QsoSwitches txAllowed />);
    const autoseq = await screen.findByRole('button', { name: 'AutoSeq' });
    await waitFor(() => expect(autoseq).toHaveAttribute('aria-pressed', 'true'));
    expect(screen.getByRole('button', { name: '1 QSO' })).toHaveAttribute('aria-pressed', 'false');
    fireEvent.click(autoseq);
    await waitFor(() => expect(postControl).toHaveBeenCalledWith('autoseq', false));
    fireEvent.click(screen.getByRole('button', { name: '1 QSO' }));
    await waitFor(() => expect(postControl).toHaveBeenCalledWith('single_qso', true));
  });

  it('shows the transmit period as Tx 00/30 or Tx 15/45 and switches between them', async () => {
    useStation.setState({ status: { mode: 'FT8' } as never });
    vi.mocked(getControls).mockResolvedValue({ tx_period: false });
    render(<QsoSwitches txAllowed />);
    const odd = await screen.findByRole('button', { name: 'Tx 15/45' });
    expect(odd).toHaveClass('chip--period-odd');
    fireEvent.click(odd);
    await waitFor(() => expect(postControl).toHaveBeenCalledWith('tx_period', true));
    useStation.setState({ status: null });
  });

  it('shows the AutoSeq level written by the engine and has the RRR box', async () => {
    useEngineTx.getState().set({ messages: ['', '', '', '', '', ''], selected: null, minute: '', autoseq: 'AutoSeq 3' });
    vi.mocked(getControls).mockResolvedValue({ autoseq: true, rrr: false });
    render(<QsoSwitches txAllowed />);
    expect(await screen.findByRole('button', { name: 'AutoSeq 3' })).toBeInTheDocument();
    const rrr = screen.getByRole('button', { name: 'RRR' });
    expect(rrr).toHaveAttribute('aria-pressed', 'false');
    fireEvent.click(rrr);
    await waitFor(() => expect(postControl).toHaveBeenCalledWith('rrr', true));
    useEngineTx.getState().set(null);
  });

  it('keeps Enable Tx and AutoTX off-limits while transmission is closed, but lets you turn them off', async () => {
    vi.mocked(getControls).mockResolvedValue({ enable_tx: true, auto_tx: false });
    render(<QsoSwitches txAllowed={false} />);
    const enable = await screen.findByRole('button', { name: 'Abilita Tx' });
    await waitFor(() => expect(enable).toHaveAttribute('aria-pressed', 'true'));
    expect(enable).toBeEnabled();
    expect(screen.getByRole('button', { name: 'AutoTX' })).toBeDisabled();
  });

  it('explains and disables the switches when the engine cannot be reached', async () => {
    vi.mocked(getControls).mockRejectedValue(new Error('502'));
    render(<QsoSwitches txAllowed />);
    expect(await screen.findByText(/comandi di Aurora non raggiungibili/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'AutoSeq' })).toBeDisabled();
  });
});
