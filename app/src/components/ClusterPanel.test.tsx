import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ClusterSpot } from '../api/client';
import { DEFAULT_CLUSTER_OPTIONS, type ClusterOptions } from '../lib/clusterOptions';

vi.mock('../api/client', () => ({
  getClusterSpots: vi.fn(),
  getCallsign: vi.fn(async () => null),
  getDxped: vi.fn(async () => ({ dxped: { fetched_ms: 1, items: [{ call: 'JA1ABC', country: 'Japan', start: '2000-01-01', end: '2999-12-31', info: '' }], error: null } })),
  getStationInfo: vi.fn().mockResolvedValue({ rig_control: true }),
  postFrequency: vi.fn().mockResolvedValue({ dial_hz: 14_074_000 }),
  ApiError: class extends Error {},
}));
import { getClusterSpots, postFrequency } from '../api/client';
import { useStation } from '../state/station';
import { ClusterPanel } from './ClusterPanel';

const spot = (id: number, call: string, band: string, mode: string, country: string): ClusterSpot => ({
  id, received_ms: Date.now(), server: 'Prova', spotter: 'W3LPL', freq_khz: 14074, dx_call: call, comment: 'FT8 -10 dB', time_utc: '1234', grid: null, band, mode, country, continent: 'EU',
});

const config = (enabled: boolean, callsign = 'IU3ABC') => ({ enabled, callsign, servers: [], login_commands: [], max_spots: 500, max_age_min: 60 });

function show(options?: ClusterOptions) {
  return render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <ClusterPanel options={options} />
    </QueryClientProvider>,
  );
}

/** I titoli delle colonne, nell'ordine in cui si vedono. */
const heads = () => screen.getAllByRole('columnheader').map((th) => th.textContent);
const saved = () => JSON.parse(localStorage.getItem('aurora.cluster.cols') ?? 'null') as { order: string[]; widths: Record<string, number> } | null;
/** jsdom non impagina: ogni titolo largo 50 px, uno dopo l'altro. */
function layoutHeads() {
  screen.getAllByRole('columnheader').forEach((th, i) => {
    th.getBoundingClientRect = () => ({ left: i * 50, right: i * 50 + 50, width: 50, top: 0, bottom: 20, height: 20, x: i * 50, y: 0, toJSON: () => ({}) });
  });
}
const pointer = (type: string, clientX: number) =>
  act(() => {
    window.dispatchEvent(new MouseEvent(type, { clientX, button: 0 }));
  });
const oneSpot = () => vi.mocked(getClusterSpots).mockResolvedValue({ config: config(true), servers: [], spots: [spot(1, 'JA1ABC', '20m', 'FT8', 'Japan')] });

beforeEach(() => {
  vi.mocked(postFrequency).mockClear();
  localStorage.clear();
  useStation.setState({ status: { dial_hz: 14_074_000, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: true, de_call: 'IU3ABC', de_grid: 'JN55' } });
});

describe('ClusterPanel', () => {
  it('says it is off and links to the settings', async () => {
    vi.mocked(getClusterSpots).mockResolvedValue({ config: config(false), servers: [], spots: [] });
    show();
    expect(await screen.findByText(/DX Cluster è spento/)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Apri le impostazioni' })).toHaveAttribute('href', '#/impostazioni/cluster');
  });

  it('asks for the callsign when it is missing', async () => {
    vi.mocked(getClusterSpots).mockResolvedValue({ config: config(true, ''), servers: [], spots: [] });
    show();
    expect(await screen.findByText(/Manca il tuo nominativo/)).toBeInTheDocument();
  });

  it('lists the spots with the country and says how many servers are connected', async () => {
    vi.mocked(getClusterSpots).mockResolvedValue({
      config: config(true),
      servers: [{ key: 'a:1', name: 'A', state: 'connected', detail: null, spots: 2, since_ms: 0 }, { key: 'b:1', name: 'B', state: 'error', detail: 'x', spots: 0, since_ms: 0 }],
      spots: [spot(2, 'JA1ABC', '20m', 'FT8', 'Japan'), spot(1, 'K1ABC', '40m', 'CW', 'United States')],
    });
    show();
    expect(await screen.findByText('JA1ABC')).toBeInTheDocument();
    expect(screen.getByText('Japan')).toBeInTheDocument();
    expect(screen.getByText('Collegato a 1 su 2')).toBeInTheDocument();
  });

  it('shows only my band when asked', async () => {
    vi.mocked(getClusterSpots).mockResolvedValue({ config: config(true), servers: [], spots: [spot(2, 'JA1ABC', '20m', 'FT8', 'Japan'), spot(1, 'K1ABC', '40m', 'CW', 'United States')] });
    show();
    await screen.findByText('K1ABC');
    fireEvent.click(screen.getByRole('button', { name: 'Banda' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'La mia banda' }));
    expect(screen.queryByText('K1ABC')).toBeNull();
    expect(screen.getByText('JA1ABC')).toBeInTheDocument();
  });

  it('a double click on a spot moves the radio to its exact frequency', async () => {
    vi.mocked(getClusterSpots).mockResolvedValue({ config: config(true), servers: [], spots: [spot(1, 'K1ABC', '40m', 'CW', 'United States')] });
    show();
    fireEvent.doubleClick(await screen.findByText('K1ABC'));
    await waitFor(() => expect(postFrequency).toHaveBeenCalledWith(14_074_000));
    // la riga «Radio portata su…» non c'e' piu': la frequenza si vede sul display
    expect(screen.queryByText(/Radio portata su/)).not.toBeInTheDocument();
  });

  it('a single click only selects: the radio does not move', async () => {
    vi.mocked(getClusterSpots).mockResolvedValue({ config: config(true), servers: [], spots: [spot(1, 'JA1ABC', '20m', 'FT8', 'Japan')] });
    show();
    fireEvent.click(await screen.findByText('JA1ABC'));
    expect(postFrequency).not.toHaveBeenCalled();
  });

  it('highlights the spots of DXpeditions on the air in the NG3K calendar', async () => {
    vi.mocked(getClusterSpots).mockResolvedValue({ config: config(true), servers: [], spots: [spot(2, 'JA1ABC', '20m', 'FT8', 'Japan'), spot(1, 'K1ABC', '40m', 'CW', 'United States')] });
    show();
    expect(await screen.findByText('JA1ABC')).toBeInTheDocument();
    expect(screen.getByText('JA1ABC').closest('tr')).toHaveClass('cluster-row--dxped');
    expect(screen.getByText('K1ABC').closest('tr')).not.toHaveClass('cluster-row--dxped');
  });

  it('draws the grid with the columns in the default order', async () => {
    oneSpot();
    show();
    await screen.findByText('JA1ABC');
    expect(screen.getByRole('table')).toHaveClass('cluster-grid');
    expect(heads()).toEqual(['DE', 'Freq', 'DX', 'Paese', 'Modo', 'Commenti', 'UTC']);
    // bandiera e nome del paese nella stessa cella: una cella per titolo
    expect(screen.getByText('JA1ABC').closest('tr')?.children).toHaveLength(7);
  });

  it('dragging a header moves the column and the order is remembered', async () => {
    oneSpot();
    const first = show();
    await screen.findByText('JA1ABC');
    layoutHeads();
    // DX (terzo titolo, 100-150 px) lasciato sulla prima meta' di DE
    fireEvent.pointerDown(screen.getByRole('columnheader', { name: 'DX' }), { button: 0, clientX: 125 });
    pointer('pointermove', 10);
    expect(screen.getByRole('columnheader', { name: 'DE' })).toHaveClass('cl-h--drop-before');
    pointer('pointerup', 10);
    expect(heads()).toEqual(['DX', 'DE', 'Freq', 'Paese', 'Modo', 'Commenti', 'UTC']);
    // le celle seguono i titoli
    expect(screen.getByText('JA1ABC').closest('td')).toBe(screen.getByText('JA1ABC').closest('tr')?.firstElementChild);
    expect(saved()?.order).toEqual(['dx', 'de', 'freq', 'country', 'mode', 'comment', 'time']);
    first.unmount();
    show();
    await screen.findByText('JA1ABC');
    expect(heads()).toEqual(['DX', 'DE', 'Freq', 'Paese', 'Modo', 'Commenti', 'UTC']);
  });

  it('a click without moving and Escape during the drag leave the order as it is', async () => {
    oneSpot();
    show();
    await screen.findByText('JA1ABC');
    layoutHeads();
    fireEvent.pointerDown(screen.getByRole('columnheader', { name: 'UTC' }), { button: 0, clientX: 325 });
    pointer('pointermove', 327);
    pointer('pointerup', 327);
    fireEvent.pointerDown(screen.getByRole('columnheader', { name: 'UTC' }), { button: 0, clientX: 325 });
    pointer('pointermove', 10);
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    });
    pointer('pointerup', 10);
    expect(heads()).toEqual(['DE', 'Freq', 'DX', 'Paese', 'Modo', 'Commenti', 'UTC']);
    expect(saved()).toBeNull();
  });

  it('Alt+arrows on a header move the column', async () => {
    oneSpot();
    show();
    await screen.findByText('JA1ABC');
    fireEvent.keyDown(screen.getByRole('columnheader', { name: 'DE' }), { key: 'ArrowRight', altKey: true });
    expect(heads()).toEqual(['Freq', 'DE', 'DX', 'Paese', 'Modo', 'Commenti', 'UTC']);
    fireEvent.keyDown(screen.getByRole('columnheader', { name: 'UTC' }), { key: 'ArrowLeft', altKey: true });
    expect(heads()).toEqual(['Freq', 'DE', 'DX', 'Paese', 'Modo', 'UTC', 'Commenti']);
    expect(saved()?.order).toEqual(['freq', 'de', 'dx', 'country', 'mode', 'time', 'comment']);
  });

  it('dragging the right edge of a header resizes the column and the width is remembered', async () => {
    oneSpot();
    const first = show();
    await screen.findByText('JA1ABC');
    const grip = screen.getByRole('columnheader', { name: 'Paese' }).querySelector('.cl-grip');
    if (!grip) throw new Error('manca la maniglia');
    fireEvent.pointerDown(grip, { button: 0, clientX: 100 });
    pointer('pointermove', 160);
    pointer('pointerup', 160);
    // 13 em di partenza + 60 px (a 16 px per em in jsdom)
    expect(saved()?.widths.country ?? 0).toBeCloseTo(16.75, 5);
    // il trascinamento del bordo non sposta la colonna
    expect(heads()).toEqual(['DE', 'Freq', 'DX', 'Paese', 'Modo', 'Commenti', 'UTC']);
    first.unmount();
    const again = show();
    await screen.findByText('JA1ABC');
    expect(again.container.querySelectorAll('col')[3]?.style.width).toBe('16.75em');
  });

  it('the header menu resets order and widths', async () => {
    localStorage.setItem('aurora.cluster.cols', JSON.stringify({ order: ['time', 'dx', 'de', 'freq', 'country', 'mode', 'comment'], widths: { dx: 20 } }));
    oneSpot();
    const { container } = show();
    await screen.findByText('JA1ABC');
    expect(heads()[0]).toBe('UTC');
    fireEvent.contextMenu(screen.getByRole('columnheader', { name: 'DX' }), { clientX: 40, clientY: 10 });
    fireEvent.click(screen.getByRole('menuitem', { name: 'Ripristina colonne' }));
    expect(heads()).toEqual(['DE', 'Freq', 'DX', 'Paese', 'Modo', 'Commenti', 'UTC']);
    expect(saved()).toEqual({ order: ['de', 'freq', 'dx', 'country', 'mode', 'comment', 'time'], widths: {} });
    expect(container.querySelectorAll('col')[2]?.style.width).toBe('7em');
    expect(screen.queryByRole('menu')).toBeNull();
  });

  it('the columns hidden by the cog are not drawn, the others keep their order', async () => {
    localStorage.setItem('aurora.cluster.cols', JSON.stringify({ order: ['dx', 'de', 'freq', 'country', 'mode', 'comment', 'time'] }));
    oneSpot();
    show({ ...DEFAULT_CLUSTER_OPTIONS, hidden: ['de', 'mode'] });
    await screen.findByText('JA1ABC');
    expect(heads()).toEqual(['DX', 'Freq', 'Paese', 'Commenti', 'UTC']);
    expect(screen.getByText('JA1ABC').closest('tr')?.children).toHaveLength(5);
  });
});
