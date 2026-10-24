import { act, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useStation } from '../state/station';
import { InputLevel } from './InputLevel';

const level = (state: 'silent' | 'low' | 'good' | 'high' | 'overdrive', peak = 0.3, rms = 0.1) => ({ type: 'audio_level' as const, peak, rms, clipped: 0, state });

beforeEach(() => {
  useStation.getState().reset();
  vi.useRealTimers();
});

describe('InputLevel', () => {
  it('shows nothing until the audio input sends a measure', () => {
    render(<InputLevel />);
    expect(screen.queryByRole('meter')).toBeNull();
  });

  it('shows the level with its state and the readings', () => {
    useStation.getState().apply(level('good'));
    render(<InputLevel />);
    const meter = screen.getByRole('meter', { name: 'Livello dell’audio in ingresso' });
    expect(meter).toHaveAttribute('aria-valuenow', '-10');
    expect(meter).toHaveAttribute('aria-valuetext', expect.stringContaining('Giusto'));
    expect(screen.getByText('Giusto')).toBeInTheDocument();
  });

  it('warns when the input is overdriven, with what to do about it', () => {
    useStation.getState().apply(level('overdrive', 1, 0.7));
    render(<InputLevel />);
    expect(screen.getByRole('status')).toHaveTextContent('Saturo');
    expect(document.querySelector('.in-level')?.getAttribute('title')).toContain('Abbassa il volume di registrazione');
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
});
