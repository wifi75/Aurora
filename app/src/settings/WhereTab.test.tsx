import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({
  getRotator: vi.fn(async () => ({ status: { state: 'connected', connected: true, az: 0, el: null, target: null, moving: false, error: null, rotctld_found: true }, config: { enabled: true, min_az: 0, max_az: 360 } })),
  ApiError: class extends Error {},
}));
import { WhereGroups } from './WhereTab';

const rotorSelect = () => screen.getAllByText('Rotore')[0]?.closest('label')?.querySelector('select') as HTMLSelectElement;

beforeEach(() => localStorage.clear());

describe('Impostazioni › Finestre: il rotore', () => {
  it('nella finestra principale è la scheda sopra la Stazione; «Nascosto» la spegne e si riaccende', async () => {
    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <WhereGroups />
      </QueryClientProvider>,
    );
    // rotore acceso nelle Impostazioni e nessuna scelta fatta: la scheda c'e'
    await waitFor(() => expect(rotorSelect().value).toBe('main'));
    fireEvent.change(rotorSelect(), { target: { value: 'hidden' } });
    expect(localStorage.getItem('aurora.station.rotor.hidden')).toBe('1');
    await waitFor(() => expect(rotorSelect().value).toBe('hidden'));
    fireEvent.change(rotorSelect(), { target: { value: 'main' } });
    expect(localStorage.getItem('aurora.station.rotor.hidden')).toBe('0');
  });

  it('in una finestra a parte: il rotore si stacca come gli altri pannelli', async () => {
    const open = vi.spyOn(window, 'open').mockReturnValue(null);
    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <WhereGroups />
      </QueryClientProvider>,
    );
    fireEvent.change(rotorSelect(), { target: { value: 'new' } });
    expect(String(open.mock.calls[0]?.[0])).toContain('items=rotator');
    await waitFor(() => expect(rotorSelect().value).toMatch(/^in:/));
    open.mockRestore();
  });
});
