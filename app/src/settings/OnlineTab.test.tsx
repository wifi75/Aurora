import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getOnline: vi.fn(), putOnline: vi.fn() }));
import { getOnline, putOnline } from '../api/client';
import { OnlineTab } from './OnlineTab';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  vi.mocked(getOnline).mockReset().mockResolvedValue({ psk: false, solar: false });
  vi.mocked(putOnline).mockReset().mockResolvedValue({ psk: true, solar: false });
});

describe('OnlineTab', () => {
  it('starts with both services off and says what each site sees', async () => {
    render(<OnlineTab />);
    expect(await screen.findByLabelText('Chi mi ha ricevuto (PSK Reporter)')).not.toBeChecked();
    expect(screen.getByLabelText('Propagazione (hamqsl.com)')).not.toBeChecked();
    expect(screen.getByText(/Il sito vede il tuo nominativo e il tuo indirizzo IP/)).toBeInTheDocument();
    expect(screen.getByText(/Il sito vede solo il tuo indirizzo IP/)).toBeInTheDocument();
  });

  it('turns one service on without touching the other', async () => {
    render(<OnlineTab />);
    fireEvent.click(await screen.findByLabelText('Chi mi ha ricevuto (PSK Reporter)'));
    await waitFor(() => expect(putOnline).toHaveBeenCalledWith({ psk: true, solar: false }));
    expect(await screen.findByText('Salvato.')).toBeInTheDocument();
  });

  it('says so when it cannot save', async () => {
    vi.mocked(putOnline).mockRejectedValue(new Error('500'));
    render(<OnlineTab />);
    fireEvent.click(await screen.findByLabelText('Propagazione (hamqsl.com)'));
    expect(await screen.findByText('Non è stato possibile salvare.')).toBeInTheDocument();
  });
});
