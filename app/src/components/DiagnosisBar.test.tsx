import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getClock: vi.fn(), getEngineSettings: vi.fn(), putEngineSettings: vi.fn(), getCat: vi.fn() }));
import { getClock, getEngineSettings, putEngineSettings } from '../api/client';
import { resetSlowPcClosed } from '../lib/diagnosis';
import { useStation } from '../state/station';
import { DiagnosisBar } from './DiagnosisBar';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

const clock = (offset_ms: number | null) => ({
  config: { enabled: true, servers: [], interval_secs: 600, warn_ms: 200, auto_correct: false },
  last: { at_ms: 1, server: 'x', offset_ms, delay_ms: 10, servers_ok: 1, servers_tried: 1, error: null, corrected: false },
});
const station = (de_call = 'IU3ABC') => useStation.getState().apply({ type: 'status', dial_hz: 14_074_000, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: false, de_call, de_grid: 'IO91wm' });

const engine = (DecodeDepth: string) => ({ version: 1, file: 'x', values: { DecodeDepth }, items: {} });

beforeEach(() => {
  useStation.getState().reset();
  resetSlowPcClosed();
  vi.mocked(getClock).mockReset().mockResolvedValue(clock(10) as never);
  vi.mocked(getEngineSettings).mockReset().mockResolvedValue(engine('3') as never);
  vi.mocked(putEngineSettings).mockReset().mockResolvedValue(engine('2') as never);
});

describe('DiagnosisBar', () => {
  it('shows nothing when everything is fine', async () => {
    station();
    render(<DiagnosisBar />);
    await vi.waitFor(() => expect(getClock).toHaveBeenCalled());
    expect(screen.queryByRole('list', { name: 'Diagnosi della stazione' })).toBeNull();
  });

  it('warns about the clock with a link to fix it', async () => {
    vi.mocked(getClock).mockResolvedValue(clock(900) as never);
    station();
    render(<DiagnosisBar />);
    expect(await screen.findByText(/L’orologio del PC è fuori di 900 ms/)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Sistema' })).toHaveAttribute('href', '#/impostazioni/clock');
  });

  it('says the contest notice of the engine in the chosen language, with the Contest page', async () => {
    station();
    useStation.getState().apply({ type: 'notice', id: 'contest', detail: "Contest: la banda 20m non e' ammessa dal regolamento del contest. Cambia banda o modo prima di trasmettere." });
    render(<DiagnosisBar />);
    expect(await screen.findByText(/Contest: la banda 20m non è ammessa dal regolamento, Aurora non trasmette/)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Sistema' })).toHaveAttribute('href', '#/contest');
    // gli avvisi del motore restano nello stato anche dopo reset(): non devono finire nelle prove che seguono
    useStation.setState({ notices: {} });
  });

  it('warns about a missing callsign and an overdriven input', async () => {
    station('');
    useStation.getState().apply({ type: 'audio_level', peak: 1, rms: 0.7, clipped: 0.2, state: 'overdrive' });
    render(<DiagnosisBar />);
    expect(await screen.findByText(/manca il tuo nominativo/)).toBeInTheDocument();
    expect(screen.getByText(/L’ingresso audio è saturo/)).toBeInTheDocument();
  });

  it('on a slow PC at Deep offers Normal and writes DecodeDepth only when pressed', async () => {
    station();
    useStation.getState().apply({ type: 'notice', id: 'slow_pc', detail: '11.7|3' });
    render(<DiagnosisBar />);
    expect(await screen.findByText(/Il PC fatica a decodificare in tempo \(11\.7 s per periodo\): passa la profondità a Normale/)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Impostazione' })).toHaveAttribute('href', '#/impostazioni/ft8decode');
    expect(putEngineSettings).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Passa a Normale' }));
    await vi.waitFor(() => expect(putEngineSettings).toHaveBeenCalledWith({ DecodeDepth: '2' }));
    // la profondita' e' cambiata: l'avviso vecchio sparisce, se il PC resta lento il motore ne manda uno nuovo con «|2»
    await vi.waitFor(() => expect(screen.queryByText(/fatica a decodificare/)).toBeNull());
  });

  it('once closed, the advice does not come back for the same depth', async () => {
    station();
    useStation.getState().apply({ type: 'notice', id: 'slow_pc', detail: '11.7|3' });
    const { unmount } = render(<DiagnosisBar />);
    await screen.findByText(/fatica a decodificare/);
    fireEvent.click(screen.getByRole('button', { name: 'Chiudi l’avviso' }));
    unmount();
    // un avviso nuovo del motore, stessa profondita', anche con la barra ricreata: niente
    useStation.getState().apply({ type: 'notice', id: 'slow_pc', detail: '12.0|3' });
    render(<DiagnosisBar />);
    await vi.waitFor(() => expect(getEngineSettings).toHaveBeenCalled());
    expect(screen.queryByText(/fatica a decodificare/)).toBeNull();
    expect(putEngineSettings).not.toHaveBeenCalled();
  });
});
