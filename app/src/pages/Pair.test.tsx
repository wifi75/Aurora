import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from '../api/client';
import { Pair } from './Pair';

vi.mock('../api/client', async (importOriginal) => {
  const original = await importOriginal<typeof import('../api/client')>();
  return { ...original, finishPairing: vi.fn() };
});

import { finishPairing } from '../api/client';

function fill(code: string, name = 'iPhone di Tiziano') {
  fireEvent.change(screen.getByLabelText('Codice di associazione'), { target: { value: code } });
  fireEvent.change(screen.getByLabelText('Nome del dispositivo'), { target: { value: name } });
  fireEvent.click(screen.getByRole('button', { name: 'Associa questo dispositivo' }));
}

describe('Pair', () => {
  beforeEach(() => {
    vi.mocked(finishPairing).mockReset();
  });

  it('blocks submit while code has fewer than 8 digits', () => {
    render(<Pair onPaired={() => {}} />);
    fill('12345');
    expect(finishPairing).not.toHaveBeenCalled();
    expect(screen.getByText('Il codice ha 8 cifre: controlla quello mostrato sul PC.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Associa questo dispositivo' })).toBeEnabled();
  });

  it('prefills the code from the QR link and keeps only digits', () => {
    render(<Pair initialCode="1234 5678" onPaired={() => {}} />);
    expect(screen.getByLabelText('Codice di associazione')).toHaveValue('12345678');
  });

  it('calls onPaired after a successful pairing', async () => {
    vi.mocked(finishPairing).mockResolvedValue();
    const onPaired = vi.fn();
    render(<Pair onPaired={onPaired} />);
    fill('12345678');
    await vi.waitFor(() => expect(onPaired).toHaveBeenCalledTimes(1));
    expect(finishPairing).toHaveBeenCalledWith('12345678', 'iPhone di Tiziano');
  });

  it('shows error and countdown on 429', async () => {
    vi.mocked(finishPairing).mockRejectedValue(new ApiError(429, 'locked', 30));
    render(<Pair onPaired={() => {}} />);
    fill('12345678');
    expect(await screen.findByText(/Troppi tentativi\. Riprova tra 30 secondi/)).toBeInTheDocument();
  });

  it('explains a wrong code and an expired code', async () => {
    vi.mocked(finishPairing).mockRejectedValueOnce(new ApiError(400, 'bad_code'));
    render(<Pair onPaired={() => {}} />);
    fill('12345678');
    expect(await screen.findByText('Codice non valido. Controlla le cifre e riprova.')).toBeInTheDocument();
    vi.mocked(finishPairing).mockRejectedValueOnce(new ApiError(410, 'expired'));
    fireEvent.click(screen.getByRole('button', { name: 'Associa questo dispositivo' }));
    expect(await screen.findByText('Il codice è scaduto. Generane uno nuovo sul PC.')).toBeInTheDocument();
  });

  it('submits by itself when opened from a link with a code', async () => {
    vi.mocked(finishPairing).mockResolvedValue();
    const onPaired = vi.fn();
    render(<Pair initialCode="12345678" autoSubmit onPaired={onPaired} />);
    await vi.waitFor(() => expect(onPaired).toHaveBeenCalledTimes(1));
    expect(finishPairing).toHaveBeenCalledTimes(1);
    expect(finishPairing).toHaveBeenCalledWith('12345678', expect.any(String));
  });

  it('does not submit by itself without the link flag', async () => {
    render(<Pair initialCode="12345678" onPaired={() => {}} />);
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(finishPairing).not.toHaveBeenCalled();
  });

  it('does not submit by itself when the code is incomplete', async () => {
    render(<Pair initialCode="1234" autoSubmit onPaired={() => {}} />);
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(finishPairing).not.toHaveBeenCalled();
  });

  it('shows the reason when the automatic attempt fails', async () => {
    vi.mocked(finishPairing).mockRejectedValue(new ApiError(410, 'expired'));
    render(<Pair initialCode="12345678" autoSubmit onPaired={() => {}} />);
    expect(await screen.findByText('Il codice è scaduto. Generane uno nuovo sul PC.')).toBeInTheDocument();
  });
});
