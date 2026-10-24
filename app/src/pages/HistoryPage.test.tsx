import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getHistory: vi.fn(), getHistoryStats: vi.fn(), clearHistory: vi.fn().mockResolvedValue(undefined) }));
import { clearHistory, getHistory, getHistoryStats, type HistoryRow } from '../api/client';
import { HistoryPage } from './HistoryPage';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

const row = (id: number, text: string, call: string | null, over: Partial<HistoryRow> = {}): HistoryRow => ({
  id, ts_ms: 1_800_000_000_000 + id * 1000, band: '20m', mode: 'FT8', dial_hz: 14_074_000, freq_hz: 1500, snr: -10, dt: 0.1, text, call, country: 'Japan', continent: 'AS', tag: 'cq', ...over,
});

beforeEach(() => {
  vi.mocked(getHistory).mockReset().mockResolvedValue({ total: 2, rows: [row(2, 'CQ JA1XYZ PM95', 'JA1XYZ'), row(1, 'CQ K1ABC FN42', 'K1ABC', { country: 'United States' })] });
  vi.mocked(getHistoryStats).mockReset().mockResolvedValue({ total: 2, oldest_ms: 1_800_000_000_000, calls: 2 });
  vi.mocked(clearHistory).mockClear();
});

describe('HistoryPage', () => {
  it('lists the messages heard with their band, mode and country', async () => {
    render(<HistoryPage />);
    expect(await screen.findByText('CQ JA1XYZ PM95')).toBeInTheDocument();
    expect(screen.getByText('CQ K1ABC FN42')).toBeInTheDocument();
    expect(screen.getByText('United States')).toBeInTheDocument();
    expect(screen.getByText('2 di 2 messaggi')).toBeInTheDocument();
    expect(getHistory).toHaveBeenCalledWith({ q: '', band: '', mode: '', period: '24h' }, 0, 100);
  });

  it('searches by text after a short pause and by band and mode at once', async () => {
    render(<HistoryPage />);
    await screen.findByText('CQ JA1XYZ PM95');
    fireEvent.change(screen.getByPlaceholderText('Nominativo o testo'), { target: { value: 'k1abc' } });
    fireEvent.change(screen.getByLabelText('Banda'), { target: { value: '40m' } });
    fireEvent.change(screen.getByLabelText('Modo'), { target: { value: 'FT4' } });
    await waitFor(() => expect(getHistory).toHaveBeenCalledWith({ q: 'k1abc', band: '40m', mode: 'FT4', period: '24h' }, 0, 100));
  });

  it('picks a station by clicking its row', async () => {
    render(<HistoryPage />);
    fireEvent.click(await screen.findByText('CQ K1ABC FN42'));
    await waitFor(() => expect(screen.getByPlaceholderText('Nominativo o testo')).toHaveValue('K1ABC'));
  });

  it('loads the next page when there are more messages', async () => {
    vi.mocked(getHistory).mockResolvedValueOnce({ total: 3, rows: [row(3, 'CQ A1A', 'A1A'), row(2, 'CQ B2B', 'B2B')] });
    vi.mocked(getHistory).mockResolvedValueOnce({ total: 3, rows: [row(1, 'CQ C3C', 'C3C')] });
    render(<HistoryPage />);
    fireEvent.click(await screen.findByRole('button', { name: 'Carica altri' }));
    expect(await screen.findByText('CQ C3C')).toBeInTheDocument();
    expect(getHistory).toHaveBeenLastCalledWith(expect.anything(), 2, 100);
    expect(screen.getByText('3 di 3 messaggi')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Carica altri' })).toBeNull();
  });

  it('says so when nothing matches and when the history is not available', async () => {
    vi.mocked(getHistory).mockResolvedValueOnce({ total: 0, rows: [] });
    const { unmount } = render(<HistoryPage />);
    expect(await screen.findByText('Nessun messaggio trovato.')).toBeInTheDocument();
    unmount();
    vi.mocked(getHistory).mockRejectedValueOnce(new Error('404'));
    render(<HistoryPage />);
    expect(await screen.findByText(/Lo storico non è disponibile/)).toBeInTheDocument();
  });

  it('clears the history only after a confirmation', async () => {
    const confirm = vi.spyOn(window, 'confirm').mockReturnValueOnce(false).mockReturnValueOnce(true);
    render(<HistoryPage />);
    const button = await screen.findByRole('button', { name: 'Svuota lo storico' });
    fireEvent.click(button);
    expect(clearHistory).not.toHaveBeenCalled();
    fireEvent.click(button);
    await waitFor(() => expect(clearHistory).toHaveBeenCalledTimes(1));
    confirm.mockRestore();
  });
});
