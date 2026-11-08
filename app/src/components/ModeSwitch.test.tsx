import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ModeSwitch } from './ModeSwitch';

const openMenu = () => fireEvent.click(screen.getByRole('button', { expanded: false }));

describe('ModeSwitch', () => {
  it('shows the current mode and opens a menu with FT8, FT4, JT65, JT9, JT4 and Q65', () => {
    render(<ModeSwitch mode="FT8" onPick={() => {}} />);
    expect(screen.getByRole('button').textContent).toContain('FT8');
    expect(screen.queryByRole('menu')).toBeNull();
    openMenu();
    expect(screen.getAllByRole('menuitemradio').map((b) => b.textContent)).toEqual(['FT8', 'FT4', 'JT65', 'JT9', 'JT4', 'Q65']);
    expect(screen.getByRole('menuitemradio', { name: 'FT8' })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('menuitemradio', { name: 'FT4' })).toHaveAttribute('aria-checked', 'false');
  });

  it('asks for the clicked mode and closes the menu', () => {
    const onPick = vi.fn();
    render(<ModeSwitch mode="FT8" onPick={onPick} />);
    openMenu();
    fireEvent.click(screen.getByRole('menuitemradio', { name: 'FT4' }));
    expect(onPick).toHaveBeenCalledWith('FT4');
    expect(screen.queryByRole('menu')).toBeNull();
  });

  it('does nothing when already in that mode', () => {
    const onPick = vi.fn();
    render(<ModeSwitch mode="FT8" onPick={onPick} />);
    openMenu();
    fireEvent.click(screen.getByRole('menuitemradio', { name: 'FT8' }));
    expect(onPick).not.toHaveBeenCalled();
  });

  it('is only a display without the radio command, and locked while transmitting', () => {
    const { rerender } = render(<ModeSwitch mode="FT8" />);
    expect(screen.getByRole('button')).toBeDisabled();
    rerender(<ModeSwitch mode="FT8" onPick={() => {}} locked />);
    expect(screen.getByRole('button')).toBeDisabled();
  });

  it('lights the JT mode in use, both for JT65+JT9 and JT65 for the old name JT', () => {
    const cases: [string, string[]][] = [['JT65', ['JT65']], ['JT9', ['JT9']], ['JT9+JT65', ['JT65', 'JT9']], ['JT', ['JT65']]];
    for (const [mode, on] of cases) {
      const { unmount } = render(<ModeSwitch mode={mode} onPick={() => {}} />);
      openMenu();
      for (const m of ['JT65', 'JT9']) {
        expect(screen.getByRole('menuitemradio', { name: m })).toHaveAttribute('aria-checked', String(on.includes(m)));
      }
      expect(screen.getByRole('menuitemradio', { name: 'FT8' })).toHaveAttribute('aria-checked', 'false');
      unmount();
    }
  });
});
