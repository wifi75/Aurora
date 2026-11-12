import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import type { ReactNode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { RotatorSnapshot, RotatorStatus } from '../../api/client';

vi.mock('../../api/client', () => ({
  getRotator: vi.fn(),
  rotatorGoto: vi.fn(),
  rotatorStop: vi.fn(),
  getDxcc: vi.fn(async () => ({})),
  ApiError: class extends Error {
    code?: string;
    detail?: string;
  },
}));
import { getRotator, rotatorGoto, rotatorStop } from '../../api/client';
import { bearingDeg, gridCenter, type Point } from '../../lib/geo';
import { publishRotorDx, readRotorDx } from '../../lib/rotorDx';
import { useStation } from '../../state/station';
import { RotatorPage } from '../../pages/RotatorPage';
import { RotorCard, type RotorPlace } from './RotorCard';
import { StationRotor } from './StationRotor';

const status = (over: Partial<RotatorStatus> = {}): RotatorStatus => ({ state: 'connected', connected: true, az: 120, el: null, target: null, moving: false, error: null, rotctld_found: true, ...over });
const snapshot = (st: RotatorStatus, enabled = true): RotatorSnapshot =>
  ({ status: st, config: { enabled, connection: 'launch', model: 603, model_name: 'Yaesu GS-232B', port: 'COM5', baud: 9600, host: '127.0.0.1', tcp_port: 4533, min_az: 0, max_az: 360 } }) as unknown as RotatorSnapshot;

function wrap(node: ReactNode) {
  return render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{node}</QueryClientProvider>);
}
const card = (place: RotorPlace, call = 'JA1ABC', grid = 'PM95') => wrap(<RotorCard call={call} grid={grid} place={place} />);
const expectedSp = () => bearingDeg(gridCenter('JN55') as Point, gridCenter('PM95') as Point);

beforeEach(() => {
  localStorage.clear();
  location.hash = '#/';
  vi.mocked(getRotator).mockReset().mockResolvedValue(snapshot(status()));
  vi.mocked(rotatorGoto).mockReset().mockImplementation(async (az: number) => status({ target: az, moving: true }));
  vi.mocked(rotatorStop).mockReset().mockResolvedValue(status());
  useStation.setState({ status: { dial_hz: 14_074_000, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: true, de_call: 'IU3ABC', de_grid: 'JN55', dx_call: 'JA1ABC', dx_grid: 'PM95' } } as never);
});

describe('RotorCard (la scheda del rotore condivisa)', () => {
  it('mostra lo stato, la direzione corta e lunga del DX e manda il rotore sul DX', async () => {
    card('page');
    expect(await screen.findByText('Collegato')).toBeTruthy();
    const sp = expectedSp();
    const dx = within(document.querySelector('.rotor-dx') as HTMLElement);
    expect(dx.getByText('JA1ABC')).toBeTruthy();
    expect(dx.getByText(`Corto ${sp}°`)).toBeTruthy();
    expect(dx.getByText(`Lungo ${(sp + 180) % 360}°`)).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'DX' }));
    await waitFor(() => expect(rotatorGoto).toHaveBeenCalledWith(sp));
    fireEvent.click(screen.getByRole('button', { name: 'Ferma' }));
    await waitFor(() => expect(rotatorStop).toHaveBeenCalled());
  });

  it('gradi a mano, ±2 e ±10 dalla posizione attuale', async () => {
    card('page');
    await screen.findByText('Collegato');
    // nella pagina Rotore le funzioni in piu' sono gia' aperte
    fireEvent.click(screen.getByRole('button', { name: '+2' }));
    await waitFor(() => expect(rotatorGoto).toHaveBeenLastCalledWith(122));
    fireEvent.click(screen.getByRole('button', { name: '−10' }));
    await waitFor(() => expect(rotatorGoto).toHaveBeenLastCalledWith(112));
    fireEvent.change(screen.getByRole('spinbutton'), { target: { value: '275' } });
    fireEvent.click(screen.getByRole('button', { name: 'Vai' }));
    await waitFor(() => expect(rotatorGoto).toHaveBeenLastCalledWith(275));
  });

  it('con il rotore spento i comandi sono disattivati e c’e’ il collegamento alle Impostazioni', async () => {
    vi.mocked(getRotator).mockResolvedValue(snapshot(status({ state: 'off', connected: false, az: null }), false));
    card('page');
    expect(await screen.findByText('Spento')).toBeTruthy();
    expect((screen.getByRole('button', { name: 'Ferma' }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole('button', { name: 'DX' }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole('button', { name: '90' }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole('button', { name: '+2' }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByRole('link', { name: 'Impostazioni del rotore' })).toBeTruthy();
  });

  it('se lo stato non si legge dice l’errore, non «spento»', async () => {
    vi.mocked(getRotator).mockRejectedValue(new Error('rete'));
    card('page');
    expect(await screen.findByText(/Impossibile leggere lo stato del rotore/)).toBeTruthy();
    expect(screen.queryByText('Spento')).toBeNull();
    expect(screen.queryByRole('link', { name: 'Impostazioni del rotore' })).toBeNull();
  });

  it('l’errore di un GO sparisce dopo uno STOP riuscito', async () => {
    vi.mocked(rotatorGoto).mockRejectedValue(new Error('no'));
    card('page');
    await screen.findByText('Collegato');
    fireEvent.click(screen.getByRole('button', { name: '90' }));
    expect(await screen.findByText('Comando non riuscito.')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Ferma' }));
    await waitFor(() => expect(screen.queryByText('Comando non riuscito.')).toBeNull());
  });

  it('sopra la pagina, nascosta, non interroga il rotore', async () => {
    localStorage.setItem('aurora.digi.rotor.hidden', '1');
    card('digi');
    await new Promise((r) => setTimeout(r, 20));
    expect(getRotator).not.toHaveBeenCalled();
    expect(document.querySelector('.rotor-card')).toBeNull();
  });

  it('senza DX lo dice, e senza il mio locatore chiede di scriverlo', async () => {
    card('page', '', '');
    expect(await screen.findByText('Nessuna stazione DX')).toBeTruthy();
    useStation.setState({ status: { de_grid: null } } as never);
    expect(await screen.findByText(/Manca il tuo locatore/)).toBeTruthy();
  });

  it('sopra la pagina ha il pulsante per staccarla in una finestra a parte; nella finestra a parte no', async () => {
    const { unmount } = card('digi');
    await screen.findByText('Collegato');
    expect(screen.getByRole('button', { name: /finestra a parte/ })).toBeTruthy();
    unmount();
    card('window');
    await screen.findByText('Collegato');
    expect(screen.queryByRole('button', { name: /finestra a parte/ })).toBeNull();
    expect(document.querySelector('.rotor-card--window')).toBeTruthy();
  });

  it('staccata da PSK/RTTY/CW: apre la finestra a parte e la pagina resta quella', async () => {
    location.hash = '#/digi';
    const open = vi.spyOn(window, 'open').mockReturnValue(null);
    card('digi');
    await screen.findByText('Collegato');
    fireEvent.click(screen.getByRole('button', { name: /finestra a parte/ }));
    fireEvent.click(screen.getByRole('menuitem', { name: 'In una finestra nuova' }));
    expect(open).toHaveBeenCalled();
    expect(String(open.mock.calls[0]?.[0])).toContain('items=rotator');
    expect(location.hash).toBe('#/digi');
    // staccata, sopra la pagina non si vede piu' (niente doppioni)
    await waitFor(() => expect(document.querySelector('.rotor-card--digi')).toBeNull());
    open.mockRestore();
  });
});

describe('StationRotor (la scheda nella Stazione, FT8, FT4, ...)', () => {
  it('si vede da sola quando il rotore e’ acceso, segue il DX della Stazione e lo scrive per la finestra a parte', async () => {
    wrap(<StationRotor />);
    expect(await screen.findByText('Collegato')).toBeTruthy();
    expect(document.querySelector('.rotor-card--station')).toBeTruthy();
    expect(screen.getByText(`Corto ${expectedSp()}°`)).toBeTruthy();
    expect(readRotorDx()).toEqual({ call: 'JA1ABC', grid: 'PM95' });
    act(() => useStation.setState({ status: { de_grid: 'JN55', dx_call: 'W1AW', dx_grid: 'FN31' } } as never));
    await waitFor(() => expect(readRotorDx()).toEqual({ call: 'W1AW', grid: 'FN31' }));
  });

  it('con il rotore spento nelle Impostazioni non compare; ✕ la nasconde e la scelta si ricorda', async () => {
    vi.mocked(getRotator).mockResolvedValue(snapshot(status({ state: 'off', connected: false }), false));
    const first = wrap(<StationRotor />);
    await waitFor(() => expect(getRotator).toHaveBeenCalled());
    expect(document.querySelector('.rotor-card')).toBeNull();
    first.unmount();
    vi.mocked(getRotator).mockResolvedValue(snapshot(status()));
    wrap(<StationRotor />);
    await screen.findByText('Collegato');
    fireEvent.click(screen.getByRole('button', { name: /Nascondi la scheda/ }));
    await waitFor(() => expect(document.querySelector('.rotor-card')).toBeNull());
    expect(localStorage.getItem('aurora.station.rotor.hidden')).toBe('1');
  });

  it('staccata in una finestra a parte non si vede nella Stazione', async () => {
    localStorage.setItem('aurora-finestre', JSON.stringify([{ id: 'f1', items: ['rotator'], dir: 'row', sizes: [1], open: true }]));
    wrap(<StationRotor />);
    await waitFor(() => expect(getRotator).toHaveBeenCalled());
    expect(document.querySelector('.rotor-card')).toBeNull();
  });
});

describe('RotatorPage (pagina Rotore e finestra a parte)', () => {
  it('la pagina ha la stessa scheda, il pulsante per la Stazione e quello per staccarla', async () => {
    wrap(<RotatorPage />);
    await screen.findByText('Collegato');
    expect(document.querySelector('.rotor-card--page')).toBeTruthy();
    expect(screen.getByRole('button', { name: /finestra a parte/ })).toBeTruthy();
    const toggle = screen.getByRole('button', { name: 'Togli dalla Stazione' });
    fireEvent.click(toggle);
    expect(await screen.findByRole('button', { name: 'Mostra nella Stazione' })).toBeTruthy();
    expect(localStorage.getItem('aurora.station.rotor.hidden')).toBe('1');
  });

  it('nella finestra a parte segue il DX scritto dalla pagina aperta (anche PSK/RTTY/CW)', async () => {
    publishRotorDx('w1aw', 'FN31');
    wrap(<RotatorPage solo />);
    await screen.findByText('Collegato');
    expect(document.querySelector('.rotor-card--window')).toBeTruthy();
    expect(screen.getByText('W1AW')).toBeTruthy();
    expect(screen.getByText('FN31')).toBeTruthy();
  });

  it('nella pagina Rotore vale il DX vivo della Stazione, non quello vecchio scritto da un’altra pagina', async () => {
    publishRotorDx('w1aw', 'FN31');
    wrap(<RotatorPage />);
    await screen.findByText('Collegato');
    const dx = within(document.querySelector('.rotor-dx') as HTMLElement);
    expect(dx.getByText('JA1ABC')).toBeTruthy();
    expect(dx.queryByText('W1AW')).toBeNull();
  });

  it('nella finestra a parte un DX scritto vuoto lascia il posto a quello vivo della Stazione', async () => {
    publishRotorDx('', '');
    wrap(<RotatorPage solo />);
    await screen.findByText('Collegato');
    expect(within(document.querySelector('.rotor-dx') as HTMLElement).getByText('JA1ABC')).toBeTruthy();
  });
});
