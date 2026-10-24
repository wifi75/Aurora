import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getControls: vi.fn(), postControl: vi.fn().mockResolvedValue(undefined) }));
import { getControls, postControl } from '../api/client';
import { QsoSwitches } from './QsoSwitches';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  vi.mocked(postControl).mockClear();
  vi.mocked(getControls).mockReset().mockResolvedValue({ autoseq: true, single_qso: false, enable_tx: false, auto_tx: false });
});

describe('QsoSwitches', () => {
  it('shows the state of the JTDX switches and sets the wanted state when one is pressed', async () => {
    render(<QsoSwitches txAllowed />);
    const autoseq = await screen.findByRole('button', { name: 'AutoSeq' });
    await waitFor(() => expect(autoseq).toHaveAttribute('aria-pressed', 'true'));
    expect(screen.getByRole('button', { name: '1 QSO' })).toHaveAttribute('aria-pressed', 'false');
    fireEvent.click(autoseq);
    await waitFor(() => expect(postControl).toHaveBeenCalledWith('autoseq', false));
    fireEvent.click(screen.getByRole('button', { name: '1 QSO' }));
    await waitFor(() => expect(postControl).toHaveBeenCalledWith('single_qso', true));
  });

  it('keeps Enable Tx and AutoTX off-limits while transmission is closed, but lets you turn them off', async () => {
    vi.mocked(getControls).mockResolvedValue({ enable_tx: true, auto_tx: false });
    render(<QsoSwitches txAllowed={false} />);
    const enable = await screen.findByRole('button', { name: 'Abilita Tx' });
    await waitFor(() => expect(enable).toHaveAttribute('aria-pressed', 'true'));
    expect(enable).toBeEnabled();
    expect(screen.getByRole('button', { name: 'AutoTX' })).toBeDisabled();
  });

  it('explains and disables the switches when JTDX cannot be reached', async () => {
    vi.mocked(getControls).mockRejectedValue(new Error('502'));
    render(<QsoSwitches txAllowed />);
    expect(await screen.findByText(/comandi di jtdx non raggiungibili/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'AutoSeq' })).toBeDisabled();
  });
});
