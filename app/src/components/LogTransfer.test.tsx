import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => {
  class ApiError extends Error {
    constructor(
      readonly status: number,
      readonly code: string,
    ) {
      super(`${status} ${code}`);
    }
  }
  return {
    ApiError,
    importLog: vi.fn(),
    newLogInfo: vi.fn(),
    startNewLog: vi.fn(),
  };
});
import { ApiError, importLog, newLogInfo, startNewLog } from '../api/client';
import { LogTransfer } from './LogTransfer';

function show() {
  return render(
    <QueryClientProvider client={new QueryClient()}>
      <LogTransfer />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  vi.mocked(importLog).mockReset();
  vi.mocked(startNewLog).mockReset();
  vi.mocked(newLogInfo).mockReset().mockResolvedValue({ qsos: 1234, file: 'C:\Aurora\logbook.db', folder: 'C:\Aurora' });
});

describe('LogTransfer', () => {
  it('sends the export to the export page', () => {
    show();
    expect(screen.getByRole('link', { name: 'Esporta ADIF' })).toHaveAttribute('href', '#/esporta');
  });

  it('imports the chosen file and tells what happened', async () => {
    vi.mocked(importLog).mockResolvedValue({ read: 10, added: 7, duplicates: 2, skipped: 1 });
    show();
    const file = new File(['<eor>'], 'wsjtx_log.adi', { type: 'text/plain' });
    await userEvent.upload(screen.getByLabelText('Importa un file ADIF'), file);
    expect(await screen.findByText(/Importati 7 QSO. Già presenti: 2. Scartati: 1. Record letti: 10./)).toBeInTheDocument();
    expect(vi.mocked(importLog)).toHaveBeenCalledWith(file);
  });

  it('explains the refusals', async () => {
    vi.mocked(importLog).mockRejectedValueOnce(new ApiError(403, 'local_only'));
    show();
    await userEvent.upload(screen.getByLabelText('Importa un file ADIF'), new File(['x'], 'a.adi'));
    expect(await screen.findByText(/solo dal PC o dalla rete di casa/)).toBeInTheDocument();
    vi.mocked(importLog).mockRejectedValueOnce(new Error('boom'));
    await userEvent.upload(screen.getByLabelText('Importa un file ADIF'), new File(['x'], 'b.adi'));
    await waitFor(() => expect(screen.getByText('Non è stato possibile importare il file.')).toBeInTheDocument());
  });

  describe('new empty log', () => {
    const open = async (first = true) => {
      if (first) show();
      await userEvent.click(screen.getByRole('button', { name: 'Nuovo log vuoto' }));
      return screen.findByRole('alertdialog');
    };

    it('warns with the QSO count, offers the export and has Cancel as the default', async () => {
      const dialog = await open();
      expect(dialog).toHaveTextContent('ATTENZIONE');
      expect(dialog).toHaveTextContent('1234 QSO');
      expect(screen.getByRole('link', { name: 'Esporta prima l’ADIF' })).toHaveAttribute('href', '#/esporta');
      expect(screen.getByRole('button', { name: 'Annulla' })).toHaveFocus();
      // Invio sul pulsante predefinito ed Esc non fanno partire niente
      await userEvent.keyboard('{Enter}');
      expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
      await open(false);
      await userEvent.keyboard('{Escape}');
      expect(screen.queryByRole('alertdialog')).not.toBeInTheDocument();
      expect(vi.mocked(startNewLog)).not.toHaveBeenCalled();
    });

    it('keeps the current log aside and tells where it went', async () => {
      vi.mocked(startNewLog).mockResolvedValue({ archived: 1234, archive: 'C:\Aurora\logbook-archivio-20261113-101500.db' });
      await open();
      await userEvent.click(screen.getByRole('button', { name: /tieni quello attuale/ }));
      expect(vi.mocked(startNewLog)).toHaveBeenCalledWith('keep');
      expect(await screen.findByText(/I 1234 QSO di prima sono in C:\Aurora\logbook-archivio-20261113-101500.db/)).toBeInTheDocument();
    });

    it('deletes only after the red second confirmation with the typed word', async () => {
      vi.mocked(startNewLog).mockResolvedValue({ deleted: 1234 });
      await open();
      await userEvent.click(screen.getByRole('button', { name: /cancella quello attuale/ }));
      expect(screen.getByRole('alertdialog')).toHaveTextContent('I 1234 QSO del log attuale andranno persi per sempre');
      const confirm = screen.getByRole('button', { name: 'Cancella 1234 QSO e crea il log nuovo' });
      expect(confirm).toBeDisabled();
      await userEvent.type(screen.getByRole('textbox'), 'cancel');
      expect(confirm).toBeDisabled();
      await userEvent.type(screen.getByRole('textbox'), 'la');
      expect(confirm).toBeEnabled();
      await userEvent.click(confirm);
      expect(vi.mocked(startNewLog)).toHaveBeenCalledWith('replace');
      expect(await screen.findByText('Log nuovo aperto. 1234 QSO cancellati.')).toBeInTheDocument();
    });

    it('explains a refusal during a QSO and a failed archive', async () => {
      vi.mocked(startNewLog).mockRejectedValueOnce(new ApiError(409, 'qso_in_progress'));
      await open();
      await userEvent.click(screen.getByRole('button', { name: /tieni quello attuale/ }));
      expect(await screen.findByText(/c’è un QSO in corso/)).toBeInTheDocument();
      vi.mocked(startNewLog).mockRejectedValueOnce(new ApiError(502, 'log_write'));
      await userEvent.click(screen.getByRole('button', { name: 'Nuovo log vuoto' }));
      await userEvent.click(await screen.findByRole('button', { name: /tieni quello attuale/ }));
      expect(await screen.findByText(/il log attuale è rimasto com’era/)).toBeInTheDocument();
    });
  });
});
