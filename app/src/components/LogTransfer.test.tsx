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
  };
});
import { ApiError, importLog } from '../api/client';
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
});
