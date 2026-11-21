import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { WefaxSnapshot } from '../api/client';

vi.mock('../api/client', () => ({
  getCat: vi.fn(() => Promise.resolve({ config: {}, status: {} })),
  getControls: vi.fn(() => Promise.resolve({})),
  postFrequency: vi.fn(() => Promise.resolve()),
  postRxFrequency: vi.fn(() => Promise.resolve()),
  postTxFrequency: vi.fn(() => Promise.resolve()),
  postCatExtra: vi.fn(() => Promise.resolve()),
  getWefax: vi.fn(),
  getWefaxImages: vi.fn(() => Promise.resolve([])),
  getWefaxCurrentBlob: vi.fn(() => Promise.reject(new Error('none'))),
  putWefax: vi.fn((s: unknown) => Promise.resolve(s)),
  wefaxRx: vi.fn(() => Promise.resolve()),
  wefaxRxPage: vi.fn(() => Promise.resolve()),
  deleteWefaxImage: vi.fn(() => Promise.resolve()),
  wefaxImageUrl: vi.fn((id: string) => `/api/v1/wefax/images/${id}`),
}));
import { getWefax, getWefaxImages, postFrequency, putWefax, wefaxRx } from '../api/client';
import { WefaxPage } from './WefaxPage';

const SNAP: WefaxSnapshot = {
  rx: { listening: true, receiving: false, state: 'apt', ioc: 576, lpm: 120, width: 1809, rows: 0, sync: 0, level_db: -40, last_image: null },
  settings: {
    ioc: 576, lpm_576: 120, lpm_288: 120, filter: 'medium', manual: false, auto_lpm: true, auto_center: true, auto_slant: true, auto_save: true, auto_receive: false,
    center_hz: 1900, shift_hz: 800, slant_ppm: 0, max_rows: 1200, correlation: 0.05, folder: '', afc: true, squelch: false, squelch_level: 30,
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
  // la cascata misura il suo riquadro: jsdom non ha ResizeObserver
  vi.stubGlobal('ResizeObserver', class { observe() {} unobserve() {} disconnect() {} });
});

describe('WefaxPage AFC e SQL', () => {
  it('shows AFC on, SQL off with its threshold, and saves the switches', async () => {
    const { unmount } = page();
    const afc = await screen.findByRole('button', { name: /^AFC/ });
    expect(afc).toHaveAttribute('aria-pressed', 'true');
    const sql = screen.getByRole('button', { name: 'SQL' });
    expect(sql).toHaveAttribute('aria-pressed', 'false');
    expect(screen.getByRole('spinbutton', { name: 'Soglia SQL' })).toHaveValue(30);
    expect(screen.getByRole('img', { name: 'Spettro dell’audio WEFAX' })).toBeInTheDocument();
    fireEvent.click(sql);
    expect(putWefax).toHaveBeenCalledWith(expect.objectContaining({ squelch: true, afc: true }));
    fireEvent.click(afc);
    expect(putWefax).toHaveBeenCalledWith(expect.objectContaining({ afc: false }));
    unmount();
  });
});

describe('WefaxPage frecce delle frequenze', () => {
  it('le frecce passano alla frequenza piu alta e piu bassa dell elenco e la radio ci va (portante 1,9 kHz sotto)', async () => {
    window.localStorage.setItem('aurora.wefax.lastPreset', 'DDH3 · DDK3 · DDK6-3855');
    const { unmount } = page();
    const up = await screen.findByRole('button', { name: 'Frequenza più alta dell’elenco' });
    const down = screen.getByRole('button', { name: 'Frequenza più bassa dell’elenco' });
    vi.mocked(postFrequency).mockClear();
    fireEvent.click(up);
    // l'elenco ordinato per frequenza: dopo 3855 kHz viene 4228 kHz (CBV)
    expect(vi.mocked(postFrequency).mock.calls[0]).toEqual([4_228_000 - 1_900, true]);
    vi.mocked(postFrequency).mockClear();
    fireEvent.click(down);
    expect(vi.mocked(postFrequency).mock.calls[0]?.[0]).toBeLessThan(4_228_000 - 1_900);
    unmount();
  });
});

describe('WefaxPage', () => {
  it('shows the viewer with zoom controls, the waiting message and the commands', async () => {
    const { unmount } = page();
    await screen.findByText(/In attesa di una cartina/);
    expect(screen.getByRole('button', { name: 'Ingrandisci' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Riduci' })).toBeInTheDocument();
    // niente «Adatta» e niente «1:1»: la cartina si vede sempre a pixel reali (100 %)
    expect(screen.queryByRole('button', { name: 'Adatta' })).toBeNull();
    expect(screen.queryByRole('button', { name: '1:1' })).toBeNull();
    expect(screen.getByText('100 %')).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: 'Segui l’immagine in arrivo' })).toBeChecked();
    expect(screen.getByRole('button', { name: 'Ferma ascolto' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Chiudi e salva' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Scarta' })).toBeDisabled();
    expect(screen.getByText('Ancora nessuna cartina ricevuta')).toBeInTheDocument();
    unmount();
  });

  it('parte a pixel reali e i pulsanti − e + cambiano la scala', async () => {
    const { unmount } = page();
    await screen.findByText(/In attesa di una cartina/);
    expect(screen.getByText('100 %')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Ingrandisci' }));
    expect(screen.getByText('150 %')).toBeInTheDocument();
    unmount();
  });

  it('sends the slant correction as a setting', async () => {
    const { unmount } = page();
    await screen.findByText(/In attesa di una cartina/);
    fireEvent.click(screen.getByRole('button', { name: '+10' }));
    expect(putWefax).toHaveBeenCalledWith(expect.objectContaining({ slant_ppm: 10, ioc: 576 }));
    unmount();
  });

  it('shows the automatic slant switch on and saves it', async () => {
    const { unmount } = page();
    await screen.findByText(/In attesa di una cartina/);
    const box = screen.getByRole('checkbox', { name: 'Inclinazione automatica' });
    expect(box).toBeChecked();
    fireEvent.click(box);
    expect(putWefax).toHaveBeenCalledWith(expect.objectContaining({ auto_slant: false, ioc: 576 }));
    unmount();
  });

  it('shows the off message and starts listening when asked', async () => {
    vi.mocked(getWefax).mockResolvedValue({ ...SNAP, rx: { ...SNAP.rx, listening: false } });
    const { unmount } = page();
    expect(await screen.findByRole('button', { name: 'Avvia ascolto' })).toBeInTheDocument();
    expect(screen.getByRole('status', { name: 'Ascolto spento' })).toBeInTheDocument();
    unmount();
  });
});
