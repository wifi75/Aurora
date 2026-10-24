import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError } from '../api/client';
import { StopButton } from './StopButton';

vi.mock('../api/client', async (importOriginal) => {
  const original = await importOriginal<typeof import('../api/client')>();
  return { ...original, postHalt: vi.fn() };
});

import { postHalt } from '../api/client';

describe('StopButton', () => {
  beforeEach(() => {
    vi.mocked(postHalt).mockReset();
  });

  it('calls postHalt once per click and confirms', async () => {
    vi.mocked(postHalt).mockResolvedValue();
    render(<StopButton />);
    fireEvent.click(screen.getByRole('button', { name: /stop tx/i }));
    expect(postHalt).toHaveBeenCalledTimes(1);
    expect(await screen.findByText('Trasmissione fermata')).toBeInTheDocument();
  });

  it('stays enabled and reports the failure when the core is unreachable', async () => {
    vi.mocked(postHalt).mockRejectedValue(new TypeError('rete assente'));
    render(<StopButton />);
    const button = screen.getByRole('button', { name: /stop tx/i });
    fireEvent.click(button);
    expect(await screen.findByText(/Non sono riuscito a fermare/)).toBeInTheDocument();
    expect(button).toBeEnabled();
  });

  it('explains an authorization problem', async () => {
    vi.mocked(postHalt).mockRejectedValue(new ApiError(401, 'unauthorized'));
    render(<StopButton />);
    fireEvent.click(screen.getByRole('button', { name: /stop tx/i }));
    expect(await screen.findByText(/Dispositivo non associato/)).toBeInTheDocument();
  });
});
