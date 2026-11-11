import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../api/client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../api/client')>()),
  getLog4om: vi.fn(),
}));
import { getLog4om } from '../../api/client';
import { DigiLog, GRID_KEY, OLD_WIDTHS_KEY, migrateWidths } from './DigiLog';

const props = {
  qso: { call: '', name: '', qth: '', wwloc: '', notes: '' },
  rsts: '599',
  rstr: '599',
  modeName: 'RTTY',
  dialHz: 14_080_000,
  audioHz: 1500,
  onSpot: () => Promise.resolve(),
  onQso: () => {},
  onRst: () => {},
  onSave: () => {},
  onClear: () => {},
  width: 400,
  onWidth: () => {},
};

beforeEach(() => {
  localStorage.clear();
  vi.mocked(getLog4om).mockReset();
});

describe('DigiLog', () => {
  it('le larghezze scelte prima delle colonne spostabili passano una volta nella disposizione nuova', () => {
    localStorage.setItem(OLD_WIDTHS_KEY, JSON.stringify({ call: 11 }));
    migrateWidths();
    expect(JSON.parse(localStorage.getItem(GRID_KEY) ?? 'null')).toMatchObject({ widths: { call: 11 } });
    // gia' fatto: non si riscrive sopra la disposizione nuova
    localStorage.setItem(GRID_KEY, JSON.stringify({ order: ['call'], widths: {} }));
    migrateWidths();
    expect(JSON.parse(localStorage.getItem(GRID_KEY) ?? 'null')).toEqual({ order: ['call'], widths: {} });
  });

  it('il log ha colonne spostabili: titoli, riga in corso e QSO seguono l’ordine salvato', async () => {
    localStorage.setItem('aurora.digi.tab', 'log');
    localStorage.setItem(GRID_KEY, JSON.stringify({ order: ['call', 'num', 'mode', 'band', 'freq', 'date', 'utc', 'name', 'qth', 'country', 'grid', 'rstS', 'rstR'], widths: {} }));
    vi.mocked(getLog4om).mockResolvedValue({ total: 1, qsos: [{ date: '2026-10-25 14:22:00Z', call: 'IU3ABC', band: '20m', mode: 'RTTY', rst_sent: '599', rst_rcvd: '579', country: 'Italy', grid: 'IO91wm' }] });
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <QueryClientProvider client={client}>
        <DigiLog {...props} />
      </QueryClientProvider>,
    );
    const call = await screen.findByText('IU3ABC');
    // le colonne di base (matita): band, country e grid restano nascoste al loro posto
    expect(screen.getAllByRole('columnheader').map((th) => th.dataset.col)).toEqual(['call', 'num', 'mode', 'freq', 'date', 'utc', 'name', 'qth', 'rstS', 'rstR']);
    expect(call.closest('tr')?.firstElementChild).toBe(call.closest('td'));
    expect(screen.getByRole('table')).toHaveClass('mv-grid');
  });
});
