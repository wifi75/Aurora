import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ModeSwitch } from './ModeSwitch';

describe('ModeSwitch', () => {
  it('offers FT8, FT4 and JT and marks the current mode', () => {
    render(<ModeSwitch mode="FT8" onPick={() => {}} />);
    expect(screen.getAllByRole('button').map((b) => b.textContent)).toEqual(['FT8', 'FT4', 'JT']);
    expect(screen.getByRole('button', { name: 'FT8' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: 'FT4' })).toHaveAttribute('aria-pressed', 'false');
  });

  it('asks for the clicked mode', () => {
    const onPick = vi.fn();
    render(<ModeSwitch mode="FT8" onPick={onPick} />);
    fireEvent.click(screen.getByRole('button', { name: 'FT4' }));
    expect(onPick).toHaveBeenCalledWith('FT4');
  });

  it('does nothing when already in that mode', () => {
    const onPick = vi.fn();
    render(<ModeSwitch mode="FT8" onPick={onPick} />);
    fireEvent.click(screen.getByRole('button', { name: 'FT8' }));
    expect(onPick).not.toHaveBeenCalled();
  });

  it('is only a display without the radio command, and locked while transmitting', () => {
    const { rerender } = render(<ModeSwitch mode="FT8" />);
    expect(screen.getAllByRole('button').every((b) => (b as HTMLButtonElement).disabled)).toBe(true);
    rerender(<ModeSwitch mode="FT8" onPick={() => {}} locked />);
    expect(screen.getAllByRole('button').every((b) => (b as HTMLButtonElement).disabled)).toBe(true);
  });

  it('shows JT as active for JT65, JT9 and JT65+JT9', () => {
    for (const mode of ['JT65', 'JT9', 'JT9+JT65']) {
      const { unmount } = render(<ModeSwitch mode={mode} onPick={() => {}} />);
      expect(screen.getByRole('button', { name: 'JT' })).toHaveAttribute('aria-pressed', 'true');
      expect(screen.getByRole('button', { name: 'FT8' })).toHaveAttribute('aria-pressed', 'false');
      unmount();
    }
  });
});
