import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getLog4om: vi.fn() }));
import { getLog4om } from '../api/client';
import { useStation } from '../state/station';
import { Log4omPanel } from './Log4omPanel';

const qso = (call: string, date: string) => ({ date, call, band: '20m', mode: 'FT4', rst_sent: '', rst_rcvd: '', country: 'Russia', grid: '' });

beforeEach(() => {
  useStation.getState().reset();
  vi.mocked(getLog4om).mockReset();
});

describe('Log4omPanel', () => {
  it('fa lampeggiare la riga di un QSO nuovo in cima e non quella iniziale', async () => {
    vi.mocked(getLog4om).mockResolvedValue({ total: 2, qsos: [qso('AA1AA', '2026-10-25 14:22:00Z'), qso('BB2BB', '2026-10-25 14:00:00Z')] });
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={client}>
        <Log4omPanel />
      </QueryClientProvider>,
    );
    const first = await screen.findByText('AA1AA');
    expect(first.closest('tr')).not.toHaveClass('log4om-row--flash');
    vi.mocked(getLog4om).mockResolvedValue({ total: 3, qsos: [qso('CC3CC', '2026-10-25 14:24:00Z'), qso('AA1AA', '2026-10-25 14:22:00Z'), qso('BB2BB', '2026-10-25 14:00:00Z')] });
    await act(async () => {
      await client.invalidateQueries({ queryKey: ['log4om'] });
    });
    const fresh = await screen.findByText('CC3CC');
    expect(fresh.closest('tr')).toHaveClass('log4om-row--flash');
    expect(screen.getByText('AA1AA').closest('tr')).not.toHaveClass('log4om-row--flash');
  });
});
