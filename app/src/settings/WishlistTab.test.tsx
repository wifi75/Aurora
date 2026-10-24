import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({
  getWishlist: vi.fn(),
  putWishlist: vi.fn().mockImplementation(async (l: unknown) => l),
  testWishlist: vi.fn().mockResolvedValue('no_url'),
}));
import { getWishlist, putWishlist, testWishlist } from '../api/client';
import { EMPTY_WISHLIST } from '../lib/wishlist';
import { WishlistTab, draftToWishlist } from './WishlistTab';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

const base = { enabled: true, calls: 'k1abc\n 3y0j ', prefixes: 'VP8', countries: 'Bulgaria\nFed. Rep. of Germany', grids: 'jn45', bands: '20M, 15m', cooldown: '15', fromDecodes: true, fromCluster: true, notifyUrl: '' };

describe('draftToWishlist', () => {
  it('turns the text of the tab into a list for the core, in capitals', () => {
    expect(draftToWishlist(base)).toEqual({
      enabled: true,
      calls: ['K1ABC', '3Y0J'],
      prefixes: ['VP8'],
      countries: ['Bulgaria', 'Fed. Rep. of Germany'],
      grids: ['JN45'],
      bands: ['20m', '15m'],
      cooldown_min: 15,
      from_decodes: true,
      from_cluster: true,
      notify_url: '',
    });
  });

  it('refuses what the core would refuse', () => {
    for (const bad of [{ calls: 'K1 ABC' }, { grids: 'JN4' }, { grids: 'ZZ99' }, { bands: 'x' }, { cooldown: '0' }, { cooldown: '241' }, { cooldown: 'a' }, { notifyUrl: 'ftp://x' }, { notifyUrl: 'https://ntfy.sh/a b' }]) {
      expect(draftToWishlist({ ...base, ...bad }), JSON.stringify(bad)).toBeNull();
    }
    expect(draftToWishlist({ ...base, notifyUrl: 'https://ntfy.sh/aurora-prova' })?.notify_url).toBe('https://ntfy.sh/aurora-prova');
  });
});

describe('WishlistTab', () => {
  beforeEach(() => {
    vi.mocked(getWishlist).mockReset().mockResolvedValue({ config: { ...EMPTY_WISHLIST, calls: ['3Y0J'] }, recent: [] });
    vi.mocked(putWishlist).mockClear();
    vi.mocked(testWishlist).mockClear();
  });

  it('shows the saved list and saves a change by itself', async () => {
    render(<WishlistTab />);
    const calls = (await screen.findByLabelText('Nominativi')) as HTMLTextAreaElement;
    expect(calls.value).toBe('3Y0J');
    fireEvent.change(calls, { target: { value: '3Y0J\nK1ABC' } });
    await waitFor(() => expect(putWishlist).toHaveBeenCalledWith(expect.objectContaining({ calls: ['3Y0J', 'K1ABC'] })), { timeout: 3000 });
  });

  it('says so when an entry is not valid and does not save it', async () => {
    render(<WishlistTab />);
    fireEvent.change(await screen.findByLabelText('Griglie'), { target: { value: 'JN4' } });
    expect(await screen.findByText(/Una voce non va/)).toBeInTheDocument();
    await new Promise((r) => setTimeout(r, 900));
    expect(putWishlist).not.toHaveBeenCalled();
  });

  it('tests the alert and says what happened', async () => {
    render(<WishlistTab />);
    fireEvent.click(await screen.findByRole('button', { name: /Prova l.avviso/ }));
    expect(await screen.findByText(/Avviso di prova mostrato nella pagina/)).toBeInTheDocument();
    vi.mocked(testWishlist).mockResolvedValueOnce('sent');
    fireEvent.click(screen.getByRole('button', { name: /Prova l.avviso/ }));
    expect(await screen.findByText(/inviato anche al telefono/)).toBeInTheDocument();
  });

  it('lists the recent alerts', async () => {
    vi.mocked(getWishlist).mockResolvedValue({
      config: EMPTY_WISHLIST,
      recent: [{ call: 'K1ABC', reason: 'call', source: 'cluster', text: '', band: '20m', freq_hz: 14_195_000, time_ms: 1 }],
    });
    render(<WishlistTab />);
    expect(await screen.findByText('K1ABC')).toBeInTheDocument();
    expect(screen.getByText(/20m/)).toBeInTheDocument();
  });
});
