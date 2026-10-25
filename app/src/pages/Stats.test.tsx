import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { StatsResponse } from '../api/client';

vi.mock('../api/client', () => ({
  ApiError: class ApiError extends Error {},
  getStats: vi.fn(),
  getDxccMatrix: vi.fn(),
  importLog: vi.fn(),
  logExportUrl: (from?: string, to?: string) => `/api/v1/log/export.adi?from=${from ?? ''}&to=${to ?? ''}`,
}));
import { getDxccMatrix, getStats } from '../api/client';
import { Stats } from './Stats';

const data: StatsResponse = {
  stats: {
    qsos: 120,
    dxcc_worked: 40,
    dxcc_total: 340,
    cq_zones_worked: 22,
    cq_zones_total: 40,
    itu_zones_worked: 50,
    itu_zones_total: 90,
    grids: 75,
    prefixes: 90,
    calls: 110,
    bands: [{ band: '20m', dxcc: 30, qsos: 80 }],
  },
  missing_dxcc: ['Atlantis', 'Lemuria'],
};

function show() {
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <Stats />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.mocked(getStats).mockReset();
  vi.mocked(getDxccMatrix).mockReset().mockRejectedValue(new Error('no'));
});

describe('Stats', () => {
  it('shows the award progress, the per-band table and the missing countries', async () => {
    vi.mocked(getStats).mockResolvedValue(data);
    show();
    expect(await screen.findByText('40 di 340')).toBeInTheDocument();
    expect(screen.getByText('22 di 40')).toBeInTheDocument();
    expect(screen.getByText('120')).toBeInTheDocument();
    expect(screen.getByRole('cell', { name: '20m' })).toBeInTheDocument();
    expect(screen.getByText('Atlantis')).toBeInTheDocument();
    expect(screen.getByText('Lemuria')).toBeInTheDocument();
  });

  it('shows the DXCC card with the total of worked countries and the log export and import', async () => {
    vi.mocked(getStats).mockResolvedValue(data);
    vi.mocked(getDxccMatrix).mockResolvedValue({
      bands: ['20m', 'U'],
      rows: ['FT8', 'FT4', 'DIG'],
      worked: 318,
      total: 340,
      qsos: 9,
      row_worked: { FT8: 300, FT4: 1, DIG: 0, ALL: 318 },
      totals: { FT8: { '20m': { countries: 250, unconfirmed: 0, state: 'worked' } } },
      countries: [],
      subject: null,
    });
    show();
    expect(await screen.findByRole('table', { name: /Scheda DXCC/ })).toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: '318' })).toBeInTheDocument();
    expect(screen.getByText('NEW ONE', { selector: 'li' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Scarica ADIF' })).toBeInTheDocument();
    expect(screen.getByLabelText('Importa un file ADIF')).toBeInTheDocument();
    expect(await screen.findByText('40 di 340')).toBeInTheDocument();
  });

  it('keeps the card and the log exchange when only the statistics are missing', async () => {
    vi.mocked(getStats).mockRejectedValue(new Error('no'));
    show();
    await waitFor(() => expect(screen.getByText(/non sono disponibili/)).toBeInTheDocument());
    expect(screen.getByRole('link', { name: 'Scarica ADIF' })).toBeInTheDocument();
  });

  it('says so when every country is worked', async () => {
    vi.mocked(getStats).mockResolvedValue({ ...data, missing_dxcc: [] });
    show();
    expect(await screen.findByText('Hai lavorato tutti i paesi.')).toBeInTheDocument();
  });

  it('explains when the statistics cannot be read', async () => {
    vi.mocked(getStats).mockRejectedValue(new Error('no'));
    show();
    await waitFor(() => expect(screen.getByText(/non sono disponibili/)).toBeInTheDocument());
  });
});
