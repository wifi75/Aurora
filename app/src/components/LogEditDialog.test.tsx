import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { LogQso } from '../api/client';

vi.mock('../api/client', async () => {
  const real = await vi.importActual<typeof import('../api/client')>('../api/client');
  return { ...real, editLogQso: vi.fn(), deleteLogQso: vi.fn(), getLog4om: vi.fn(), getLogQso: vi.fn() };
});
vi.mock('../lib/toast', () => ({ toast: vi.fn() }));
import { ApiError, deleteLogQso, editLogQso, getLog4om, getLogQso } from '../api/client';
import { toast } from '../lib/toast';
import { useStation } from '../state/station';
import { LOG_COLS_KEY, Log4omPanel } from './Log4omPanel';
import { LogEditDialog, logEditErrors, logEditPatch, type LogEditValues } from './LogEditDialog';

const QSO: LogQso = { id: 7, date: '2026-10-27', time: '12:34:56', call: 'K1ABX', band: '20m', mode: 'FT8', freq_hz: 14_074_000, rst_sent: '-08', rst_rcvd: '-12', grid: 'FN42', country: 'United States', name: '', qth: '', comment: 'FT8  Sent: -08  Rcvd: -12' };
const VALUES: LogEditValues = { call: 'K1ABX', date: '2026-10-27', time: '12:34:56', band: '20m', freq: '14.074000', mode: 'FT8', rst_sent: '-08', rst_rcvd: '-12', grid: 'FN42', name: '', qth: '', comment: 'FT8  Sent: -08  Rcvd: -12' };

function show(onClose = vi.fn()) {
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <LogEditDialog qso={QSO} onClose={onClose} />
    </QueryClientProvider>,
  );
  return onClose;
}

beforeEach(() => {
  vi.mocked(editLogQso).mockReset();
  vi.mocked(deleteLogQso).mockReset();
  vi.mocked(getLog4om).mockReset();
  vi.mocked(getLogQso).mockReset();
  vi.mocked(toast).mockReset();
  localStorage.removeItem(LOG_COLS_KEY);
  useStation.getState().reset();
});

describe('logEditErrors', () => {
  it('accepts a good QSO and an empty or a 4, 6, 8 character grid', () => {
    expect(logEditErrors(VALUES)).toEqual([]);
    for (const grid of ['', 'IO91', 'IO91wm', 'IO91WM37']) expect(logEditErrors({ ...VALUES, grid })).toEqual([]);
    expect(logEditErrors({ ...VALUES, time: '12:34', freq: '' })).toEqual([]);
  });

  it('names the fields that are wrong: callsign, date and time, grid, frequency', () => {
    expect(logEditErrors({ ...VALUES, call: '  ' })).toEqual(['call']);
    expect(logEditErrors({ ...VALUES, call: 'K1 ABC' })).toEqual(['call']);
    expect(logEditErrors({ ...VALUES, date: '2026-02-30' })).toEqual(['time']);
    expect(logEditErrors({ ...VALUES, date: '' })).toEqual(['time']);
    expect(logEditErrors({ ...VALUES, time: '24:00:00' })).toEqual(['time']);
    for (const grid of ['ZZ99', 'IO9', 'IO91w', 'IO91wm3']) expect(logEditErrors({ ...VALUES, grid })).toEqual(['grid']);
    expect(logEditErrors({ ...VALUES, freq: 'quattordici' })).toEqual(['freq']);
    expect(logEditErrors({ ...VALUES, call: '', grid: 'X' })).toEqual(['call', 'grid']);
  });
});

describe('logEditPatch', () => {
  it('sends only what changed', () => {
    expect(logEditPatch(QSO, VALUES)).toEqual({});
    expect(logEditPatch(QSO, { ...VALUES, call: ' k1abc ', grid: 'fn42ab', comment: '' })).toEqual({ call: 'K1ABC', grid: 'FN42AB', comment: '' });
    expect(logEditPatch(QSO, { ...VALUES, time: '12:40:00' })).toEqual({ date: '2026-10-27', time: '12:40:00' });
    expect(logEditPatch(QSO, { ...VALUES, band: '40m', freq: '7,074' })).toEqual({ band: '40m', freq_hz: 7_074_000 });
  });
});

describe('LogEditDialog', () => {
  it('shows the QSO, saves only the corrected fields and closes', async () => {
    vi.mocked(editLogQso).mockResolvedValue({ ...QSO, call: 'K1ABC', name: 'Joe' });
    const onClose = show();
    const dialog = screen.getByRole('dialog', { name: 'Correggi o cancella il QSO' });
    const call = within(dialog).getByLabelText('Nominativo');
    expect(call).toHaveValue('K1ABX');
    expect(within(dialog).getByLabelText('Data')).toHaveValue('2026-10-27');
    expect(within(dialog).getByLabelText('Ora')).toHaveValue('12:34:56');
    expect(within(dialog).getByLabelText('Banda')).toHaveValue('20m');
    expect(within(dialog).getByLabelText('Frequenza (MHz)')).toHaveValue('14.074000');
    expect(within(dialog).getByLabelText('Modo')).toHaveValue('FT8');
    expect(within(dialog).getByLabelText('Commento')).toHaveValue('FT8  Sent: -08  Rcvd: -12');
    // la finestra dice che i log esterni non ricevono nulla
    expect(within(dialog).getByText(/Ai log esterni \(UDP, Wavelog, ClubLog, QRZ, LoTW\) non si rimanda nulla/)).toBeInTheDocument();
    await userEvent.clear(call);
    await userEvent.type(call, 'k1abc');
    await userEvent.type(within(dialog).getByLabelText('Nome'), 'Joe');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Salva' }));
    await waitFor(() => expect(onClose).toHaveBeenCalledWith('saved'));
    expect(vi.mocked(editLogQso)).toHaveBeenCalledWith(7, { call: 'K1ABC', name: 'Joe' });
    expect(vi.mocked(toast)).toHaveBeenCalledWith('QSO con K1ABC corretto.', 'ok');
  });

  it('does not save a wrong value and says which one', async () => {
    vi.mocked(editLogQso).mockResolvedValue(QSO);
    const onClose = show();
    await userEvent.clear(screen.getByLabelText('Nominativo'));
    await userEvent.type(screen.getByLabelText('Locatore'), '9');
    fireEvent.change(screen.getByLabelText('Data'), { target: { value: '' } });
    await userEvent.click(screen.getByRole('button', { name: 'Salva' }));
    expect(screen.getAllByRole('alert').map((a) => a.textContent)).toEqual([
      'Scrivi il nominativo: da 3 a 13 lettere, cifre o «/».',
      'Data o ora non valide.',
      'Locatore non valido: 4, 6 o 8 caratteri (per esempio IO91wm), oppure vuoto.',
    ]);
    expect(screen.getByLabelText('Nominativo')).toHaveAttribute('aria-invalid', 'true');
    expect(vi.mocked(editLogQso)).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    // corretti i valori gli avvisi spariscono e il salvataggio parte
    await userEvent.type(screen.getByLabelText('Nominativo'), 'K1ABC');
    await userEvent.clear(screen.getByLabelText('Locatore'));
    fireEvent.change(screen.getByLabelText('Data'), { target: { value: '2026-10-28' } });
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Salva' }));
    await waitFor(() => expect(onClose).toHaveBeenCalledWith('saved'));
    expect(vi.mocked(editLogQso)).toHaveBeenCalledWith(7, { call: 'K1ABC', date: '2026-10-28', time: '12:34:56', grid: '' });
  });

  it('closes without asking the core when nothing changed, with Cancel and with Esc', async () => {
    const onClose = show();
    await userEvent.click(screen.getByRole('button', { name: 'Salva' }));
    await userEvent.click(screen.getByRole('button', { name: 'Annulla' }));
    await userEvent.keyboard('{Escape}');
    expect(onClose.mock.calls).toEqual([[null], [null], [null]]);
    expect(vi.mocked(editLogQso)).not.toHaveBeenCalled();
  });

  it('deletes only after the confirmation', async () => {
    vi.mocked(deleteLogQso).mockResolvedValue();
    const onClose = show();
    await userEvent.click(screen.getByRole('button', { name: 'Elimina' }));
    const ask = await screen.findByRole('alertdialog');
    expect(ask).toHaveTextContent('Eliminare il QSO con K1ABX del 2026-10-27? Non si può recuperare dal programma. Nei log esterni resta.');
    await userEvent.click(within(ask).getByRole('button', { name: 'Annulla' }));
    expect(vi.mocked(deleteLogQso)).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: 'Elimina' }));
    await userEvent.click(within(await screen.findByRole('alertdialog')).getByRole('button', { name: 'OK' }));
    await waitFor(() => expect(onClose).toHaveBeenCalledWith('deleted'));
    expect(vi.mocked(deleteLogQso)).toHaveBeenCalledWith(7);
    expect(vi.mocked(toast)).toHaveBeenCalledWith('QSO con K1ABX eliminato.', 'ok');
  });

  it('says when the correction is refused and stays open', async () => {
    vi.mocked(editLogQso).mockRejectedValue(new ApiError(403, 'local_only'));
    const onClose = show();
    await userEvent.type(screen.getByLabelText('Nome'), 'Joe');
    await userEvent.click(screen.getByRole('button', { name: 'Salva' }));
    await waitFor(() => expect(vi.mocked(toast)).toHaveBeenCalledWith('Si può fare solo dal PC o dalla rete di casa.', 'error'));
    expect(onClose).not.toHaveBeenCalled();
  });
});

describe('Log4omPanel, correzione dei QSO', () => {
  const row = (call: string, id?: number) => ({ date: '2026-10-27 12:34:00Z', call, band: '20m', mode: 'FT8', rst_sent: '-08', rst_rcvd: '-12', country: 'United States', grid: 'FN42', id });
  const panel = () =>
    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <Log4omPanel />
      </QueryClientProvider>,
    );

  it('opens the window with a double click when the log is Aurora’s own', async () => {
    vi.mocked(getLog4om).mockResolvedValue({ total: 1, source: 'aurora', qsos: [row('K1ABX', 7)] });
    vi.mocked(getLogQso).mockResolvedValue(QSO);
    panel();
    await userEvent.dblClick(await screen.findByText('K1ABX'));
    expect(vi.mocked(getLogQso)).toHaveBeenCalledWith(7);
    expect(await screen.findByRole('dialog', { name: 'Correggi o cancella il QSO' })).toBeInTheDocument();
    expect(screen.queryByText(/sola lettura/)).not.toBeInTheDocument();
  });

  it('says the Log4OM2 log is read only and opens nothing', async () => {
    vi.mocked(getLog4om).mockResolvedValue({ total: 1, source: 'log4om', qsos: [row('K1ABX')] });
    panel();
    await userEvent.dblClick(await screen.findByText('K1ABX'));
    expect(screen.getByText('Log di Log4OM2, in sola lettura: i QSO si correggono e si cancellano in Log4OM2.')).toBeInTheDocument();
    expect(vi.mocked(getLogQso)).not.toHaveBeenCalled();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});
