import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { LogQso, LogQsos } from '../api/client';

vi.mock('../api/client', async () => {
  const real = await vi.importActual<typeof import('../api/client')>('../api/client');
  return { ...real, getLogQsos: vi.fn(), saveLogAdif: vi.fn(), editLogQso: vi.fn(), deleteLogQso: vi.fn() };
});
vi.mock('../lib/toast', () => ({ toast: vi.fn() }));
import { compactIds, deleteLogQso, editLogQso, getLogQsos, saveLogAdif } from '../api/client';
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
  vi.mocked(editLogQso).mockReset();
  vi.mocked(deleteLogQso).mockReset();
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

  it('opens the correction window with a double click or with «Modifica» and shows the corrected row', async () => {
    vi.mocked(getLogQsos).mockResolvedValueOnce(LOG).mockResolvedValue({ ...LOG, qsos: [q(3, 'K1ABD', '2026-10-27', '40m'), ...LOG.qsos.slice(1)] });
    vi.mocked(editLogQso).mockResolvedValue(q(3, 'K1ABD', '2026-10-27', '40m'));
    show();
    await userEvent.dblClick(await screen.findByText('DL2XYZ'));
    const first = screen.getByRole('dialog', { name: 'Correggi o cancella il QSO' });
    expect(within(first).getByLabelText('Nominativo')).toHaveValue('DL2XYZ');
    await userEvent.click(within(first).getByRole('button', { name: 'Annulla' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    // il doppio clic non cambia la selezione (due clic: tolta e rimessa)
    expect(screen.getByText('3 selezionati su 3 mostrati (3 nel log)')).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Modifica il QSO con K1ABC' }));
    const dialog = screen.getByRole('dialog');
    const call = within(dialog).getByLabelText('Nominativo');
    await userEvent.clear(call);
    await userEvent.type(call, 'K1ABD');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Salva' }));
    expect(vi.mocked(editLogQso)).toHaveBeenCalledWith(3, { call: 'K1ABD' });
    // la finestra si chiude e la riga si rilegge dal log
    expect(await screen.findByText('K1ABD')).toBeInTheDocument();
    expect(screen.queryByText('K1ABC')).not.toBeInTheDocument();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(screen.getByText('3 selezionati su 3 mostrati (3 nel log)')).toBeInTheDocument();
  });

  it('deletes a QSO from the window after the confirmation and forgets it in the selection', async () => {
    vi.mocked(getLogQsos).mockResolvedValueOnce(LOG).mockResolvedValue({ ...LOG, total: 2, qsos: LOG.qsos.slice(1) });
    vi.mocked(deleteLogQso).mockResolvedValue();
    show();
    await screen.findByText('K1ABC');
    await userEvent.click(screen.getByRole('button', { name: 'Modifica il QSO con K1ABC' }));
    await userEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Elimina' }));
    await userEvent.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'OK' }));
    await waitFor(() => expect(vi.mocked(deleteLogQso)).toHaveBeenCalledWith(3));
    expect(await screen.findByText('2 selezionati su 2 mostrati (2 nel log)')).toBeInTheDocument();
    expect(screen.queryByText('K1ABC')).not.toBeInTheDocument();
  });
});
