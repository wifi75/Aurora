import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { WefaxSnapshot } from '../api/client';

vi.mock('../api/client', () => ({
  getWefax: vi.fn(),
  getWefaxImages: vi.fn(() => Promise.resolve([])),
  getWefaxCurrentBlob: vi.fn(() => Promise.reject(new Error('none'))),
  putWefax: vi.fn((s: unknown) => Promise.resolve(s)),
  wefaxRx: vi.fn(() => Promise.resolve()),
  wefaxRxPage: vi.fn(() => Promise.resolve()),
  deleteWefaxImage: vi.fn(() => Promise.resolve()),
  wefaxImageUrl: vi.fn((id: string) => `/api/v1/wefax/images/${id}`),
}));
import { getWefax, getWefaxImages, putWefax, wefaxRx } from '../api/client';
import { WefaxPage } from './WefaxPage';

const SNAP: WefaxSnapshot = {
  rx: { listening: true, receiving: false, state: 'apt', ioc: 576, lpm: 120, width: 1809, rows: 0, sync: 0, level_db: -40, last_image: null },
  settings: {
    ioc: 576, lpm_576: 120, lpm_288: 120, filter: 'medium', manual: false, auto_lpm: true, auto_center: true, auto_save: true, auto_receive: false,
    center_hz: 1900, shift_hz: 800, slant_ppm: 0, max_rows: 1200, correlation: 0.05, folder: '',
  },
  folder: 'C:\\Aurora\\wefax',
  iocs: [
    { ioc: 576, width: 1809, apt_start_hz: 300, default_lpm: 120 },
    { ioc: 288, width: 904, apt_start_hz: 675, default_lpm: 120 },
  ],
  lpm_values: [240, 120, 90, 60],
  filters: ['narrow', 'medium', 'wide'],
};

function page() {
  return render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <WefaxPage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  localStorage.clear();
  vi.mocked(getWefax).mockReset().mockResolvedValue(SNAP);
  vi.mocked(putWefax).mockClear();
  vi.mocked(wefaxRx).mockClear();
  vi.mocked(getWefaxImages).mockClear();
  // jsdom non disegna: niente contesto 2D (e niente avvisi «not implemented»)
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
});

describe('WefaxPage', () => {
  it('shows the viewer with zoom controls, the waiting message and the commands', async () => {
    const { unmount } = page();
    await screen.findByText(/In attesa di una cartina/);
    expect(screen.getByRole('button', { name: 'Ingrandisci' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Riduci' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Adatta' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: '1:1' })).toHaveAttribute('aria-pressed', 'false');
    expect(screen.getByRole('checkbox', { name: 'Segui l’immagine in arrivo' })).toBeChecked();
    expect(screen.getByRole('button', { name: 'Ferma ascolto' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Chiudi e salva' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Scarta' })).toBeDisabled();
    expect(screen.getByText('Ancora nessuna cartina ricevuta')).toBeInTheDocument();
    unmount();
  });

  it('switches between fit and real pixels', async () => {
    const { unmount } = page();
    await screen.findByText(/In attesa di una cartina/);
    fireEvent.click(screen.getByRole('button', { name: '1:1' }));
    expect(screen.getByRole('button', { name: '1:1' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: 'Adatta' })).toHaveAttribute('aria-pressed', 'false');
    expect(screen.getByText('100 %')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Ingrandisci' }));
    expect(screen.getByText('150 %')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Adatta' }));
    expect(screen.getByRole('button', { name: 'Adatta' })).toHaveAttribute('aria-pressed', 'true');
    unmount();
  });

  it('sends the slant correction as a setting', async () => {
    const { unmount } = page();
    await screen.findByText(/In attesa di una cartina/);
    fireEvent.click(screen.getByRole('button', { name: '+10' }));
    expect(putWefax).toHaveBeenCalledWith(expect.objectContaining({ slant_ppm: 10, ioc: 576 }));
    unmount();
  });

  it('shows the off message and starts listening when asked', async () => {
    vi.mocked(getWefax).mockResolvedValue({ ...SNAP, rx: { ...SNAP.rx, listening: false } });
    const { unmount } = page();
    expect(await screen.findByRole('button', { name: 'Avvia ascolto' })).toBeInTheDocument();
    expect(screen.getByText('Ascolto spento')).toBeInTheDocument();
    unmount();
  });
});
