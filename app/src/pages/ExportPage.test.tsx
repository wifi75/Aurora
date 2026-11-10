import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { LogQso, LogQsos } from '../api/client';

vi.mock('../api/client', async () => {
  const real = await vi.importActual<typeof import('../api/client')>('../api/client');
  return { ...real, getLogQsos: vi.fn(), saveLogAdif: vi.fn() };
});
vi.mock('../lib/toast', () => ({ toast: vi.fn() }));
import { compactIds, getLogQsos, saveLogAdif } from '../api/client';
import { toast } from '../lib/toast';
import { ExportPage, filterQsos } from './ExportPage';

function q(id: number, call: string, date: string, band = '20m', mode = 'FT8'): LogQso {
  return { id, date, time: '12:34:56', call, band, mode, freq_hz: 14_074_000, rst_sent: '-08', rst_rcvd: '-12', grid: 'JN45', country: 'Italy' };
}

const LOG: LogQsos = { total: 3, save_dialog: true, qsos: [q(3, 'K1ABC', '2026-10-27', '40m'), q(2, 'DL2XYZ', '2026-10-26', '20m', 'FT4'), q(1, 'JA1AAA', '2026-10-25')] };

function show() {
  return render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <ExportPage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.mocked(getLogQsos).mockReset();
  vi.mocked(saveLogAdif).mockReset();
  vi.mocked(toast).mockReset();
});

describe('filterQsos', () => {
  it('keeps the QSOs matching callsign, dates, band and mode', () => {
    const all = LOG.qsos;
    expect(filterQsos(all, { call: 'k1', from: '', to: '', band: '', mode: '' }).map((x) => x.id)).toEqual([3]);
    expect(filterQsos(all, { call: '', from: '2026-10-26', to: '2026-10-26', band: '', mode: '' }).map((x) => x.id)).toEqual([2]);
    expect(filterQsos(all, { call: '', from: '', to: '', band: '20m', mode: 'FT8' }).map((x) => x.id)).toEqual([1]);
  });
});

describe('compactIds', () => {
  it('writes runs as ranges', () => {
    expect(compactIds([10, 1, 2, 3, 7, 9])).toBe('1-3,7,9-10');
    expect(compactIds([])).toBe('');
  });
});

describe('ExportPage', () => {
  it('says when the log is empty', async () => {
    vi.mocked(getLogQsos).mockResolvedValue({ total: 0, qsos: [], save_dialog: true });
    show();
    expect(await screen.findByText(/non ha ancora QSO/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Salva ADIF…' })).not.toBeInTheDocument();
  });

  it('starts with everything selected and saves all of it with the Windows window', async () => {
    vi.mocked(getLogQsos).mockResolvedValue(LOG);
    vi.mocked(saveLogAdif).mockResolvedValue({ ok: true, cancelled: false, path: 'C:\\log\\aurora.adi', count: 3 });
    show();
    expect(await screen.findByText('3 selezionati su 3 mostrati (3 nel log)')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Salva ADIF…' }));
    expect(vi.mocked(saveLogAdif)).toHaveBeenCalledWith({ all: true }, 'adif');
    await waitFor(() => expect(vi.mocked(toast)).toHaveBeenCalledWith('Salvati 3 QSO in C:\\log\\aurora.adi', 'ok'));
  });

  it('saves only the chosen QSOs and tells when the save is cancelled', async () => {
    vi.mocked(getLogQsos).mockResolvedValue(LOG);
    vi.mocked(saveLogAdif).mockResolvedValue({ ok: true, cancelled: true });
    show();
    await screen.findByText('K1ABC');
    await userEvent.click(screen.getByRole('button', { name: 'Nessuno' }));
    expect(screen.getByRole('button', { name: 'Salva ADIF…' })).toBeDisabled();
    await userEvent.click(screen.getByLabelText('Seleziona il QSO con DL2XYZ'));
    await userEvent.click(screen.getByText('JA1AAA'));
    expect(screen.getByText('2 selezionati su 3 mostrati (3 nel log)')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Salva ADIF…' }));
    expect(vi.mocked(saveLogAdif).mock.calls[0]?.[0]).toEqual({ ids: expect.arrayContaining([1, 2]) as number[] });
    await waitFor(() => expect(vi.mocked(toast)).toHaveBeenCalledWith('Salvataggio annullato.', 'info'));
  });

  it('filters by callsign and selects only what is shown', async () => {
    vi.mocked(getLogQsos).mockResolvedValue(LOG);
    show();
    await screen.findByText('K1ABC');
    await userEvent.click(screen.getByRole('button', { name: 'Nessuno' }));
    await userEvent.type(screen.getByRole('searchbox', { name: 'Nominativo' }), 'dl2');
    expect(screen.queryByText('K1ABC')).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Seleziona tutti' }));
    expect(screen.getByText('1 selezionati su 1 mostrati (3 nel log)')).toBeInTheDocument();
  });

  it('offers the download from a phone, with the chosen ids', async () => {
    vi.mocked(getLogQsos).mockResolvedValue({ ...LOG, save_dialog: false });
    show();
    await screen.findByText('K1ABC');
    await userEvent.click(screen.getByLabelText('Seleziona il QSO con K1ABC'));
    const link = screen.getByRole('link', { name: 'Scarica ADIF' });
    expect(link.getAttribute('href')).toBe('/api/v1/log/export.adi?ids=1-2');
    expect(link.getAttribute('download')).toMatch(/^aurora_log_\d{8}\.adi$/);
  });
});
