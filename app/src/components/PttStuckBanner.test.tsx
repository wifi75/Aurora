import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ postHalt: vi.fn() }));
vi.mock('../lib/soundBank', () => ({ playSound: vi.fn(() => true) }));
import { postHalt } from '../api/client';
import { playSound } from '../lib/soundBank';
import { useStation } from '../state/station';
import { PTT_STUCK_SOUND_MS, PttAbandonedNotice, PttStuckBanner, pttStuckAlert } from './PttStuckBanner';

const status = (tx_alerts: unknown[] = []) =>
  useStation.getState().apply({ type: 'status', dial_hz: 14_074_000, mode: 'FT8', tx_enabled: true, transmitting: false, decoding: false, de_call: 'IU3ABC', de_grid: 'IO91wm', tx_alerts } as never);
const stuck = (attempts = 5, secs = 4) => ({ kind: 'ptt_stuck', detail: 'la connessione con la radio si e’ interrotta', value: attempts, limit: secs });

beforeEach(() => {
  vi.useFakeTimers();
  useStation.getState().reset();
  vi.mocked(postHalt).mockReset().mockResolvedValue(undefined);
  vi.mocked(playSound).mockClear();
});
afterEach(() => vi.useRealTimers());

describe('PttStuckBanner', () => {
  it('finds the alert only when the engine reports it', () => {
    expect(pttStuckAlert(null)).toBeNull();
    status([{ kind: 'high_swr', value: 3, limit: 2.5 }]);
    expect(pttStuckAlert(useStation.getState().status)).toBeNull();
    status([stuck()]);
    expect(pttStuckAlert(useStation.getState().status)?.value).toBe(5);
  });

  it('shows nothing while the PTT is fine', () => {
    status();
    render(<PttStuckBanner />);
    expect(screen.queryByRole('alert')).toBeNull();
    expect(playSound).not.toHaveBeenCalled();
  });

  it('shows the red warning with the reason, rings, and keeps ringing until it is ignored', () => {
    status([stuck(7, 12)]);
    render(<PttStuckBanner />);
    const box = screen.getByRole('alert');
    expect(box).toHaveTextContent('Il PTT non si è spento: la radio potrebbe essere in trasmissione. Controllala o spegnila a mano.');
    expect(box).toHaveTextContent('7 tentativi in 12 s');
    expect(playSound).toHaveBeenCalledTimes(1);
    act(() => void vi.advanceTimersByTime(PTT_STUCK_SOUND_MS * 2 + 10));
    expect(playSound).toHaveBeenCalledTimes(3);
    fireEvent.click(screen.getByRole('button', { name: 'Ignora' }));
    expect(screen.queryByRole('alert')).toBeNull();
    act(() => void vi.advanceTimersByTime(PTT_STUCK_SOUND_MS * 2));
    expect(playSound).toHaveBeenCalledTimes(3);
  });

  it('asks for another attempt with the retry button', () => {
    status([stuck()]);
    render(<PttStuckBanner />);
    fireEvent.click(screen.getByRole('button', { name: 'Riprova a spegnere' }));
    expect(postHalt).toHaveBeenCalledTimes(1);
  });

  it('disappears when the PTT is confirmed off, and a new failure shows it again even after Ignore', () => {
    status([stuck()]);
    render(<PttStuckBanner />);
    fireEvent.click(screen.getByRole('button', { name: 'Ignora' }));
    act(() => status());
    expect(screen.queryByRole('alert')).toBeNull();
    act(() => status([stuck(1, 0)]));
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });
});

describe('PttAbandonedNotice', () => {
  const abandoned = { kind: 'ptt_abandoned', detail: 'la catena nuova comanda un altra radio' };

  it('shows from the status, rings once, closes with a click and survives a page reload', () => {
    status();
    render(<PttAbandonedNotice />);
    expect(screen.queryByRole('alert')).toBeNull();
    act(() => status([abandoned]));
    expect(screen.getByRole('alert')).toHaveTextContent('Il PTT della radio precedente non risulta spento: controlla la radio a mano.');
    expect(playSound).toHaveBeenCalledTimes(1);
    act(() => void vi.advanceTimersByTime(PTT_STUCK_SOUND_MS * 3));
    expect(playSound).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole('button', { name: /chiudi/i }));
    expect(screen.queryByRole('alert')).toBeNull();
    // ricaricando la pagina lo stato porta ancora l'avviso: ricompare
    cleanup();
    render(<PttAbandonedNotice />);
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('shows again for a second abandon after the first alert was cleared', () => {
    status([abandoned]);
    render(<PttAbandonedNotice />);
    fireEvent.click(screen.getByRole('button', { name: /chiudi/i }));
    act(() => status());
    act(() => status([abandoned]));
    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(playSound).toHaveBeenCalledTimes(2);
  });
});
