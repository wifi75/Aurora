import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({
  getPendingQsos: vi.fn(),
  answerPendingQso: vi.fn(async () => undefined),
  ApiError: class extends Error {},
}));
import { answerPendingQso, getPendingQsos } from '../api/client';
import { LogPrompt } from './LogPrompt';

const pending = {
  id: 7,
  qso: { call: 'K1ABC', grid: 'FN42', band: '20m', mode: 'FT8', freq_hz: 14_074_000, rst_sent: '-10', rst_rcvd: '-12', time_on_unix_ms: 0, name: '', qth: '', comment: '' },
};

function show() {
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <LogPrompt />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.mocked(getPendingQsos).mockReset();
  vi.mocked(answerPendingQso).mockClear();
});

describe('LogPrompt', () => {
  it('shows nothing when no QSO is waiting', async () => {
    vi.mocked(getPendingQsos).mockResolvedValue([]);
    show();
    await waitFor(() => expect(getPendingQsos).toHaveBeenCalled());
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('asks to log a finished QSO and sends only the corrected fields', async () => {
    vi.mocked(getPendingQsos).mockResolvedValue([pending]);
    show();
    expect(await screen.findByRole('dialog')).toBeInTheDocument();
    expect(screen.getByText('Registrare il QSO con K1ABC?')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Rapporto ricevuto'), { target: { value: '-05' } });
    fireEvent.change(screen.getByLabelText('Nome'), { target: { value: 'Joe' } });
    fireEvent.click(screen.getByRole('button', { name: 'Registra' }));
    await waitFor(() => expect(answerPendingQso).toHaveBeenCalledWith(7, 'log', { rst_rcvd: '-05', name: 'Joe' }));
  });

  it('discards a QSO without logging it', async () => {
    vi.mocked(getPendingQsos).mockResolvedValue([pending]);
    show();
    fireEvent.click(await screen.findByRole('button', { name: 'Scarta' }));
    await waitFor(() => expect(answerPendingQso).toHaveBeenCalledWith(7, 'discard', {}));
  });
});
