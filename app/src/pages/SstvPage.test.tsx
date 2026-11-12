import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { SstvSnapshot } from '../api/client';

vi.mock('../api/client', () => ({
  getSstv: vi.fn(),
  getSstvImages: vi.fn(() => Promise.resolve([])),
  getSstvImageBlob: vi.fn(),
  getSstvCurrentBlob: vi.fn(() => Promise.reject(new Error('none'))),
  sstvCurrentCleanUrl: vi.fn((order: number, smooth: number, v: number) => `/api/v1/sstv/rx/current/clean?order=${order}&smooth=${smooth}&v=${v}`),
  getCallsign: vi.fn(),
  postFrequency: vi.fn(() => Promise.resolve()),
  sstvRx: vi.fn(() => Promise.resolve()),
  sstvForce: vi.fn(() => Promise.resolve()),
  addLogQso: vi.fn(() => Promise.resolve()),
}));
// il compositore e le cronologie non servono qui: si guarda la disposizione della ricezione
vi.mock('../components/sstv/Composer', () => ({ Composer: () => <div data-testid="composer" /> }));
vi.mock('../components/sstv/History', () => ({ History: () => <div data-testid="history" /> }));
vi.mock('../components/FreqPicker', () => ({ FreqPicker: () => null }));
vi.mock('../lib/settingsWindow', () => ({ openSettingsWindow: vi.fn() }));
import { getCallsign, getSstv, sstvRx, type CallsignInfo } from '../api/client';
import { emitSstv } from '../lib/sstvBus';
import { openSettingsWindow } from '../lib/settingsWindow';
import { usePrefs } from '../lib/prefs';
import { emitSpectrum, spectrumListenerCount } from '../lib/spectrumFeed';
import { SstvPage } from './SstvPage';

const SNAP: SstvSnapshot = {
  rx: { listening: true, receiving: false, mode: null, width: 320, height: 256, rows: 0, sync: 0, offset_hz: 0, level_db: -40, last_image: null },
  tx: { state: 'idle', mode: null, progress: 0, elapsed_s: 0, total_s: 0, message: null },
  settings: { default_mode: 'm1', auto_save: true, save_tx: false, auto_receive: false, lock_mode: null, folder: '', tx_folder: '', rx_offset_hz: 0, tx_offset_hz: 0, slant_ppm: 0 },
  folder: 'C:\\Aurora\\history',
  can_transmit: true,
  modes: [{ id: 'm1', name: 'Martin 1', width: 320, height: 256, vis: 44, seconds: 114, color: 'rgb' }],
};

function page() {
  return render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <SstvPage />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  localStorage.clear();
  usePrefs.getState().reset();
  vi.mocked(getSstv).mockReset().mockResolvedValue(SNAP);
  vi.mocked(openSettingsWindow).mockReset();
  // jsdom non disegna: niente contesto 2D (e niente avvisi «not implemented»)
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
});

describe('SstvPage, ricezione', () => {
  it('puts the thin spectrum in the image column and the small log next to the mode card', async () => {
    const { container, unmount } = page();
    await screen.findByText('In attesa di un’immagine: Aurora riconosce il modo dal codice VIS o, senza VIS, dal ritmo delle righe.');
    const rx = container.querySelector('.sstv-rx');
    expect(rx).toHaveClass('sstv-rx--log');
    // lo spettro sta nella colonna del visore, sotto lo schermo: stessi bordi della scheda dell'immagine
    const stage = rx?.querySelector(':scope > .sstv-stage');
    const spectrum = screen.getByTestId('sstv-spectrum');
    expect(stage?.contains(spectrum)).toBe(true);
    expect(stage?.querySelector('.sstv-screen')?.compareDocumentPosition(spectrum)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    expect(within(spectrum).getByText('Rx 1200')).toBeInTheDocument();
    // niente piu' cascata
    expect(container.querySelector('.digi-wf')).toBeNull();
    // il log e' la terza colonna, subito dopo la scheda del modo
    const log = screen.getByRole('region', { name: 'Log QSO' });
    expect(log.parentElement).toBe(rx);
    expect(log.previousElementSibling).toHaveClass('sstv-side');
    for (const name of ['Nominativo', 'Inviato', 'Ricevuto', 'Nome', 'QTH', 'Locatore', 'Note']) expect(within(log).getByLabelText(name)).toBeInTheDocument();
    expect(within(log).getByRole('button', { name: 'Metti a log' })).toBeInTheDocument();
    expect(within(log).getByRole('button', { name: 'Pulisci' })).toBeInTheDocument();
    // lo spettro riceve le righe e smette all'uscita
    expect(spectrumListenerCount()).toBe(1);
    expect(() => act(() => emitSpectrum(new Uint8Array(4096).fill(40), 0, 3000))).not.toThrow();
    unmount();
    expect(spectrumListenerCount()).toBe(0);
  });

  it('keeps only the operating controls on the page; the settings are behind the gear', async () => {
    page();
    await screen.findByText('Martin 1');
    for (const name of ['TX', 'Ferma ascolto', 'Scarta immagine', 'Ricevi ora']) expect(screen.getByRole('button', { name })).toBeInTheDocument();
    expect(screen.queryByLabelText('Ingrandimento morbido')).toBeNull();
    expect(screen.queryByText('Impostazioni SSTV', { selector: 'summary' })).toBeNull();
    expect(screen.queryByText('Cartella delle immagini')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Impostazioni SSTV' }));
    expect(openSettingsWindow).toHaveBeenCalledWith('sstv');
  });

  it('draws the fixed tuning marks on the spectrum and the estimated offset when known', async () => {
    vi.mocked(getSstv).mockResolvedValue({ ...SNAP, rx: { ...SNAP.rx, rows: 40, offset_hz: 35 } });
    const { container } = page();
    await screen.findByText('Martin 1');
    const labels = [...container.querySelectorAll('.sstv-mark:not(.sstv-mark--found) i')].map((i) => i.textContent);
    expect(labels).toEqual(['1200', '1500', '2300']);
    expect(container.querySelectorAll('.sstv-mark--vis')).toHaveLength(3);
    expect(container.querySelector('.sstv-mark--found')).toHaveTextContent('+35 Hz');
  });

  it('shows the FSK ID callsign next to the picture and puts it in the log', async () => {
    vi.mocked(getCallsign).mockResolvedValue({ call: 'IU3ABC', dxcc: null, person: null, lookup: 'off', subscriber: null, message: null });
    const { container } = page();
    await screen.findByText('Martin 1');
    act(() => emitSstv({ type: 'sstv', kind: 'rx_fskid', call: 'IU3ABC', image_id: 'x' }));
    expect(container.querySelector('.sstv-fskid')).toHaveTextContent('IU3ABC');
    expect(container.querySelector('.sstv-fskid-text')).toHaveTextContent('IU3ABC');
    const log = screen.getByRole('region', { name: 'Log QSO' });
    expect(within(log).getByLabelText('Nominativo')).toHaveValue('IU3ABC');
    // un nominativo scritto a mano non lo copre l'FSK ID dell'immagine dopo
    fireEvent.change(within(log).getByLabelText('Nominativo'), { target: { value: 'IU3XYZ' } });
    act(() => emitSstv({ type: 'sstv', kind: 'rx_fskid', call: 'IU3DEF' }));
    expect(within(log).getByLabelText('Nominativo')).toHaveValue('IU3XYZ');
    // un'immagine nuova toglie il nominativo di quella di prima
    act(() => emitSstv({ type: 'sstv', kind: 'rx_start', mode: 'm1', width: 320, height: 256, message: 'sync' }));
    expect(container.querySelector('.sstv-fskid')).toBeNull();
    expect(screen.getByText('dal ritmo delle righe (senza VIS)')).toBeInTheDocument();
  });

  it('looks the callsign up and fills name, QTH and locator without touching what was typed', async () => {
    const info: CallsignInfo = {
      call: 'IU3ABC',
      dxcc: { country: 'Italy', continent: 'EU', prefix: 'I', cq_zone: 15, itu_zone: 28 },
      person: { fname: 'Mario', name: 'Rossi', city: 'Padova', country: 'Italy', grid: 'IO91wm', source: 'QRZ.com' },
      lookup: 'found',
      subscriber: true,
      message: null,
    };
    vi.mocked(getCallsign).mockReset().mockResolvedValue(info);
    page();
    await screen.findByText('Martin 1');
    const log = screen.getByRole('region', { name: 'Log QSO' });
    fireEvent.change(within(log).getByLabelText('Nome'), { target: { value: 'Mario Rossi' } });
    fireEvent.change(within(log).getByLabelText('Nominativo'), { target: { value: 'iu3abc' } });
    await screen.findByText('Italy · Dati da QRZ.com', {}, { timeout: 2000 });
    expect(getCallsign).toHaveBeenCalledWith('IU3ABC');
    expect(within(log).getByLabelText('Nome')).toHaveValue('Mario Rossi');
    expect(within(log).getByLabelText('QTH')).toHaveValue('Padova');
    expect(within(log).getByLabelText('Locatore')).toHaveValue('IO91wm');
  });

  it('keeps listening while the page is open, unless the operator stops it', async () => {
    vi.mocked(sstvRx).mockClear();
    vi.mocked(getSstv).mockResolvedValue({ ...SNAP, rx: { ...SNAP.rx, listening: false } });
    page();
    await screen.findByText('Martin 1');
    // l'accensione dell'apertura e quella che rimedia allo stato «spento»
    await vi.waitFor(() => expect(vi.mocked(sstvRx).mock.calls.filter((c) => c[0] === 'start').length).toBeGreaterThanOrEqual(2));
  });

  it('shows the picture being received cleaned from noise on request', async () => {
    vi.mocked(getSstv).mockResolvedValue({ ...SNAP, rx: { ...SNAP.rx, rows: 120, receiving: true } });
    const { container } = page();
    await screen.findByText('Martin 1');
    expect(container.querySelector('.sstv-canvas--clean')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Pulita' }));
    const img = container.querySelector('img.sstv-canvas--clean');
    expect(img?.getAttribute('src')).toContain('/sstv/rx/current/clean?order=3&smooth=0.4');
    expect(screen.getByText('Ordine 3')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Originale' }));
    expect(container.querySelector('.sstv-canvas--clean')).toBeNull();
  });

  it('shows the picture smoothed when the setting is on', async () => {
    const { container } = page();
    await screen.findByText('Martin 1');
    expect(container.querySelector('.sstv-canvas')).toHaveClass('sstv-canvas--crisp');
    act(() => usePrefs.getState().set({ sstvSmooth: true }));
    expect(container.querySelector('.sstv-canvas')).not.toHaveClass('sstv-canvas--crisp');
  });
});
