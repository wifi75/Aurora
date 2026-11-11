import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getLog4om: vi.fn() }));
import { getLog4om } from '../api/client';
import { useStation } from '../state/station';
import { LOG_COLS_KEY, Log4omPanel } from './Log4omPanel';

const qso = (call: string, date: string) => ({ date, call, band: '20m', mode: 'FT4', rst_sent: '', rst_rcvd: '', country: 'Russia', grid: '' });

beforeEach(() => {
  localStorage.removeItem(LOG_COLS_KEY);
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
    // la riga lampeggia quando Aurora mette a log il QSO (evento «logged»), non quando Log4OM lo scrive
    vi.mocked(getLog4om).mockResolvedValue({ total: 3, qsos: [qso('CC3CC', '2026-10-25 14:24:00Z'), qso('AA1AA', '2026-10-25 14:22:00Z'), qso('BB2BB', '2026-10-25 14:00:00Z')] });
    await act(async () => {
      useStation.setState({ justLogged: [{ call: 'CC3CC', band: '20m', mode: 'FT4', rst_sent: '', rst_rcvd: '', time_ms: (14 * 3600 + 24 * 60) * 1000, at: Date.now() }] });
      await client.invalidateQueries({ queryKey: ['log4om'] });
    });
    const fresh = await screen.findByText('CC3CC');
    expect(fresh.closest('tr')).toHaveClass('log4om-row--flash');
    expect(screen.getByText('AA1AA').closest('tr')).not.toHaveClass('log4om-row--flash');
  });

  it('ha le colonne a griglia spostabili: le celle seguono l’ordine salvato dei titoli', async () => {
    localStorage.setItem(LOG_COLS_KEY, JSON.stringify({ order: ['country', 'call', 'date', 'band', 'mode', 'rst'], widths: { call: 9 } }));
    vi.mocked(getLog4om).mockResolvedValue({ total: 1, qsos: [{ ...qso('AA1AA', '2026-10-25 14:22:00Z'), rst_sent: '-10', rst_rcvd: '-12' }] });
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const { container } = render(
      <QueryClientProvider client={client}>
        <Log4omPanel />
      </QueryClientProvider>,
    );
    const call = await screen.findByText('AA1AA');
    expect(screen.getAllByRole('columnheader').map((th) => th.dataset.col)).toEqual(['country', 'call', 'date', 'band', 'mode', 'rst']);
    const cells = Array.from(call.closest('tr')?.children ?? []);
    expect(cells[0]).toHaveClass('lg-c--country');
    expect(cells[1]).toBe(call);
    expect(cells[5]).toHaveTextContent('-10/-12');
    // la larghezza scelta a mano viene dal <colgroup>; il paese, elastico, prende lo spazio che avanza
    expect(Array.from(container.querySelectorAll('col')).map((c) => c.style.width)).toEqual(['', '9em', '6.6em', '3.8em', '3.8em', '4.8em']);
  });
});
