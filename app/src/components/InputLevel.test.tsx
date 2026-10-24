import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getInputLevel: vi.fn(), putInputLevel: vi.fn() }));
import { getInputLevel, putInputLevel } from '../api/client';
import { useStation } from '../state/station';
import { InputLevel } from './InputLevel';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

const level = (state: 'silent' | 'low' | 'good' | 'high' | 'overdrive', peak = 0.3, rms = 0.1) => ({ type: 'audio_level' as const, peak, rms, clipped: 0, state });
const volume = { device: 'Ingresso di prova', volume_pct: 80, auto: false };

beforeEach(() => {
  useStation.getState().reset();
  vi.useRealTimers();
  vi.mocked(getInputLevel).mockReset().mockResolvedValue(volume);
  vi.mocked(putInputLevel).mockReset().mockResolvedValue(volume);
});

describe('InputLevel', () => {
  it('shows nothing until the audio input sends a measure', () => {
    render(<InputLevel />);
    expect(screen.queryByRole('meter')).toBeNull();
  });

  it('shows the level with its state and the readings', async () => {
    useStation.getState().apply(level('good'));
    render(<InputLevel />);
    const meter = screen.getByRole('meter', { name: 'Livello dell’audio in ingresso' });
    expect(meter).toHaveAttribute('aria-valuenow', '-10');
    expect(meter).toHaveAttribute('aria-valuetext', expect.stringContaining('Giusto'));
    expect(screen.getByText('Giusto')).toBeInTheDocument();
    expect(await screen.findByText('80%')).toBeInTheDocument();
  });

  it('warns when the input is overdriven, with what to do about it', async () => {
    useStation.getState().apply(level('overdrive', 1, 0.7));
    render(<InputLevel />);
    expect(screen.getByRole('status')).toHaveTextContent('Saturo');
    expect(document.querySelector('.in-level')?.getAttribute('title')).toContain('Abbassa il volume di registrazione');
    await screen.findByText('80%');
  });

  it('hides again when the measures stop arriving', () => {
    vi.useFakeTimers();
    useStation.getState().apply(level('good'));
    render(<InputLevel />);
    expect(screen.getByRole('meter')).toBeInTheDocument();
    act(() => {
      vi.advanceTimersByTime(7000);
    });
    expect(screen.queryByRole('meter')).toBeNull();
  });

  it('sets the Windows recording volume from the slider', async () => {
    useStation.getState().apply(level('good'));
    render(<InputLevel />);
    const slider = await screen.findByRole('slider', { name: 'Volume di registrazione di Windows' });
    fireEvent.keyDown(slider, { key: 'ArrowDown' });
    fireEvent.keyUp(slider, { key: 'ArrowDown' });
    await waitFor(() => expect(putInputLevel).toHaveBeenCalledWith({ volume_pct: 79 }));
  });

  it('turns AUTO on and off', async () => {
    useStation.getState().apply(level('good'));
    render(<InputLevel />);
    const auto = await screen.findByRole('button', { name: 'AUTO' });
    expect(auto).toHaveAttribute('aria-pressed', 'false');
    fireEvent.click(auto);
    await waitFor(() => expect(putInputLevel).toHaveBeenCalledWith({ auto: true }));
  });

  it('keeps the meter but drops the slider when the volume cannot be adjusted', async () => {
    vi.mocked(getInputLevel).mockRejectedValue(new Error('404'));
    useStation.getState().apply(level('good'));
    render(<InputLevel />);
    await waitFor(() => expect(getInputLevel).toHaveBeenCalled());
    expect(screen.getByRole('meter')).toBeInTheDocument();
    expect(screen.queryByRole('slider')).toBeNull();
    expect(screen.queryByRole('button', { name: 'AUTO' })).toBeNull();
  });
});
