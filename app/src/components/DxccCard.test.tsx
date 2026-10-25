import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DxccMatrix } from '../lib/dxccCard';

vi.mock('../api/client', () => ({ getDxccMatrix: vi.fn() }));
import { getDxccMatrix } from '../api/client';
import { DxccCard, DxccLive } from './DxccCard';
import { DxccCountries } from './DxccCountries';

const BANDS = ['160m', '80m', '60m', '40m', '30m', '20m', '17m', '15m', '12m', '10m', '6m', '4m', 'U'];

function matrix(over: Partial<DxccMatrix> = {}): DxccMatrix {
  return {
    bands: BANDS,
    rows: ['FT8', 'FT4', 'DIG'],
    worked: 318,
    total: 340,
    qsos: 5000,
    row_worked: { FT8: 300, FT4: 120, DIG: 10, ALL: 318 },
    totals: { FT8: { '20m': { countries: 250, unconfirmed: 0, state: 'worked' }, '40m': { countries: 90, unconfirmed: 5, state: 'unconfirmed' } } },
    countries: [{ id: 1, name: 'Atlantis', prefix: 'AT', continent: 'EU', cells: { FT8: { '20m': 'worked' }, ALL: { '20m': 'worked' } } }],
    subject: null,
    ...over,
  };
}

const subject = (labels: Partial<Record<string, boolean>>) => ({
  call: 'AT9ZZ',
  country: 'Atlantis',
  prefix: 'AT',
  continent: 'EU',
  grid: 'FN42',
  band: '20m',
  mode: 'FT8',
  labels: { new_one: false, new_band: false, new_mode: false, new_grid: false, grid_band: false, grid_mode: false, ...labels },
  cells: { FT8: { '20m': 'worked' as const, '15m': 'unconfirmed' as const } },
});

function wrap(ui: ReactNode) {
  return render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  vi.mocked(getDxccMatrix).mockReset();
});

describe('DxccCard', () => {
  it('shows the six labels off, the total of worked countries and the counts per cell when no station is chosen', async () => {
    vi.mocked(getDxccMatrix).mockResolvedValue(matrix());
    wrap(<DxccCard />);
    const grid = await screen.findByRole('table', { name: /Scheda DXCC/ });
    expect(within(grid).getByRole('columnheader', { name: '318' })).toBeInTheDocument();
    for (const text of ['NEW ONE', 'NEW BAND', 'NEW MODE', 'NEW GRID', 'GRID BAND', 'GRID MODE']) {
      const el = screen.getByText(text, { selector: 'li' });
      expect(el).toHaveAttribute('data-on', 'false');
      expect(el.getAttribute('title')).toBeTruthy();
    }
    const row = within(grid).getByRole('row', { name: /FT8/ });
    const cells = within(row).getAllByRole('cell');
    expect(cells).toHaveLength(13);
    expect(cells[5]).toHaveAttribute('data-kind', 'worked');
    expect(cells[5]).toHaveTextContent('250');
    expect(cells[3]).toHaveAttribute('data-kind', 'unconfirmed');
    expect(cells[0]).toHaveAttribute('data-kind', 'none');
    expect(screen.getByText(/318 paesi lavorati su 340/)).toBeInTheDocument();
  });

  it('lights the labels of the station and colours the cells of its country', async () => {
    vi.mocked(getDxccMatrix).mockImplementation(async (q) => (q?.call ? matrix({ subject: subject({ new_band: true, grid_mode: true }) }) : matrix()));
    wrap(<DxccCard call="AT9ZZ" grid="FN42" band="20m" mode="FT8" />);
    await waitFor(() => expect(screen.getByText('NEW BAND', { selector: 'li' })).toHaveAttribute('data-on', 'true'));
    expect(screen.getByText('GRID MODE', { selector: 'li' })).toHaveAttribute('data-on', 'true');
    expect(screen.getByText('NEW ONE', { selector: 'li' })).toHaveAttribute('data-on', 'false');
    expect(screen.getAllByText(/Atlantis/)[0]).toBeInTheDocument();
    const row = within(screen.getByRole('table', { name: /Scheda DXCC/ })).getByRole('row', { name: /FT8/ });
    const cells = within(row).getAllByRole('cell');
    expect(cells[5]).toHaveAttribute('data-kind', 'worked');
    expect(cells[7]).toHaveAttribute('data-kind', 'unconfirmed');
    expect(cells[3]).toHaveAttribute('data-kind', 'none');
    // il core riceve banda e modo della radio e chiede solo le etichette
    expect(vi.mocked(getDxccMatrix)).toHaveBeenCalledWith({ call: 'AT9ZZ', grid: 'FN42', band: '20m', mode: 'FT8', compact: true });
  });

  it('can switch to the totals and tries a typed callsign', async () => {
    vi.mocked(getDxccMatrix).mockImplementation(async (q) => (q?.call ? matrix({ subject: subject({}) }) : matrix()));
    wrap(<DxccCard call="AT9ZZ" />);
    await screen.findAllByText(/Atlantis/);
    await userEvent.click(screen.getByRole('button', { name: 'Totali' }));
    const grid = screen.getByRole('table', { name: /Scheda DXCC/ });
    expect(within(within(grid).getByRole('row', { name: /FT8/ })).getAllByRole('cell')[5]).toHaveTextContent('250');
    await userEvent.type(screen.getByRole('textbox', { name: 'Prova' }), 'k1abc');
    await waitFor(() => expect(vi.mocked(getDxccMatrix)).toHaveBeenCalledWith(expect.objectContaining({ call: 'K1ABC', grid: undefined, compact: true })));
  });

  it('says when the card is not available', async () => {
    vi.mocked(getDxccMatrix).mockRejectedValue(new Error('no'));
    wrap(<DxccCard />);
    expect(await screen.findByText(/non è disponibile/)).toBeInTheDocument();
  });
});

describe('DxccLive', () => {
  it('stays off without a station and does not ask the core', () => {
    wrap(<DxccLive call="" />);
    expect(screen.getByText('NEW ONE', { selector: 'li' })).toHaveAttribute('data-on', 'false');
    expect(getDxccMatrix).not.toHaveBeenCalled();
  });

  it('asks only for the labels of the DX station on the radio band and mode', async () => {
    vi.mocked(getDxccMatrix).mockResolvedValue(matrix({ countries: undefined, subject: subject({ new_one: true, new_band: true, new_mode: true }) }));
    wrap(<DxccLive call="at9zz" grid="FN42" dialHz={14_074_000} mode="FT8" />);
    await waitFor(() => expect(screen.getByText('NEW ONE', { selector: 'li' })).toHaveAttribute('data-on', 'true'));
    expect(vi.mocked(getDxccMatrix)).toHaveBeenCalledWith({ call: 'AT9ZZ', grid: 'FN42', band: '20m', mode: 'FT8', compact: true });
    expect(screen.getByText('Atlantis')).toBeInTheDocument();
  });

  it('shows nothing when the card is not available', async () => {
    vi.mocked(getDxccMatrix).mockRejectedValue(new Error('no'));
    const { container } = wrap(<DxccLive call="AT9ZZ" />);
    await waitFor(() => expect(container.querySelector('.dxcc-live')).toBeNull());
  });
});

describe('DxccCountries', () => {
  it('lists the worked countries per band and filters them', async () => {
    const m = matrix({
      countries: [
        { id: 1, name: 'Atlantis', prefix: 'AT', continent: 'EU', cells: { FT8: { '20m': 'worked' }, ALL: { '20m': 'worked' } } },
        { id: 2, name: 'Lemuria', prefix: 'LM', continent: 'NA', cells: { ALL: { '40m': 'unconfirmed' } } },
      ],
    });
    wrap(<DxccCountries matrix={m} />);
    expect(screen.getByText('2 paesi')).toBeInTheDocument();
    await userEvent.type(screen.getByRole('searchbox'), 'lem');
    expect(screen.queryByRole('rowheader', { name: /Atlantis/ })).toBeNull();
    const lemuria = screen.getByRole('rowheader', { name: /Lemuria/ }).closest('tr');
    expect(within(lemuria as HTMLElement).getAllByRole('cell')[3]).toHaveAttribute('data-kind', 'unconfirmed');
    await userEvent.clear(screen.getByRole('searchbox'));
    await userEvent.selectOptions(screen.getByRole('combobox'), 'FT8');
    expect(screen.getByText('1 paesi')).toBeInTheDocument();
    await userEvent.selectOptions(screen.getByRole('combobox'), 'DIG');
    expect(screen.getByText('Nessun paese in questo modo.')).toBeInTheDocument();
  });
});
