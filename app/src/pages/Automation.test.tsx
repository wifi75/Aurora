import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AutomationState } from '../api/automation';

vi.mock('../api/client', () => ({ getAutomation: vi.fn(), putAutomation: vi.fn() }));
import { getAutomation, putAutomation } from '../api/client';
import { Automation } from './Automation';

const base: AutomationState = {
  enabled: false,
  mode: 'dry_run',
  live_allowed: false,
  rules: { reply_to: ['new_dxcc'], min_snr: -18, excluded_calls: [], max_attempts_per_call: 2, reply_to_wanted: false },
  limits: { max_consecutive_cycles: 6, max_per_day: 40, dead_man_secs: 120, run_unattended: false },
  replies_today: 0,
  consecutive: 0,
  log: [{ at: 1_000_000, kind: 'would_reply', call: 'AT1AA', decode_id: 4, reason: 'Risponderei a AT1AA: paese nuovo (prova a vuoto)' }],
};

function show() {
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <Automation />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.mocked(getAutomation).mockReset().mockResolvedValue(base);
  vi.mocked(putAutomation).mockReset().mockResolvedValue({ ...base, enabled: true });
});

describe('Automation', () => {
  it('shows the state, the safeguards and the decision log with the reason', async () => {
    show();
    expect(await screen.findByText(/Risponderei a AT1AA/)).toBeInTheDocument();
    expect(screen.getByText('Prova a vuoto')).toBeInTheDocument();
    expect(screen.getByText(/6 risposte di fila/)).toBeInTheDocument();
    expect(screen.getByText(/40 al giorno/)).toBeInTheDocument();
  });

  it('turns it on in dry run from the page', async () => {
    show();
    fireEvent.click(await screen.findByRole('button', { name: /accendi/i }));
    await waitFor(() => expect(putAutomation).toHaveBeenCalledWith({ enabled: true }));
  });

  it('says that the real mode is chosen on the PC and that transmission is closed', async () => {
    show();
    expect(await screen.findByText(/modo reale si attiva dal PC/i)).toBeInTheDocument();
    expect(screen.getByText(/trasmissione è chiusa/i)).toBeInTheDocument();
  });

  it('saves the rules', async () => {
    show();
    fireEvent.change(await screen.findByLabelText(/segnale minimo/i), { target: { value: '-12' } });
    fireEvent.click(screen.getByRole('button', { name: /salva le regole/i }));
    await waitFor(() => expect(putAutomation).toHaveBeenCalledWith({ rules: { ...base.rules, min_snr: -12 } }));
  });

  it('turns on the replies to the wishlist and saves them with the rules', async () => {
    show();
    fireEvent.click(await screen.findByLabelText(/rispondi anche ai cq della lista dei desiderati/i));
    fireEvent.click(screen.getByRole('button', { name: /salva le regole/i }));
    await waitFor(() => expect(putAutomation).toHaveBeenCalledWith({ rules: { ...base.rules, reply_to_wanted: true } }));
  });

  it('shows the error when the automation is unavailable', async () => {
    vi.mocked(getAutomation).mockRejectedValue(new Error('404'));
    show();
    expect(await screen.findByText(/automazione non disponibile/i)).toBeInTheDocument();
  });
});
