import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ModeSwitch } from './ModeSwitch';

const openMenu = () => fireEvent.click(screen.getByRole('button', { expanded: false }));

describe('ModeSwitch', () => {
  it('shows the current mode and opens a menu with FT8, FT4, JT65, JT9, JT4, Q65, MSK144 and WSPR', () => {
    render(<ModeSwitch mode="FT8" onPick={() => {}} />);
    expect(screen.getByRole('button').textContent).toContain('FT8');
    expect(screen.queryByRole('menu')).toBeNull();
    openMenu();
    expect(screen.getAllByRole('menuitemradio').map((b) => b.textContent)).toEqual(['FT8', 'FT4', 'JT65', 'JT9', 'JT4', 'Q65', 'MSK144', 'WSPR']);
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
    // in trasmissione non si spegne di colore: resta attivo all'aspetto, ma il clic non apre il menu
    rerender(<ModeSwitch mode="FT8" onPick={() => {}} locked />);
    const button = screen.getByRole('button');
    expect(button).not.toBeDisabled();
    expect(button).toHaveAttribute('aria-disabled', 'true');
    fireEvent.click(button);
    expect(button).toHaveAttribute('aria-expanded', 'false');
  });

  it('shows period and submode next to Q65 and asks for the full name', () => {
    const onPick = vi.fn();
    render(<ModeSwitch mode="Q65-30A" onPick={onPick} />);
    const [period, sub] = screen.getAllByRole('combobox') as [HTMLSelectElement, HTMLSelectElement];
    expect(period.value).toBe('30');
    expect(sub.value).toBe('A');
    fireEvent.change(period, { target: { value: '60' } });
    expect(onPick).toHaveBeenLastCalledWith('Q65-60A');
    fireEvent.change(sub, { target: { value: 'D' } });
    expect(onPick).toHaveBeenLastCalledWith('Q65-30D');
    openMenu();
    expect(screen.getByRole('menuitemradio', { name: 'Q65' })).toHaveAttribute('aria-checked', 'true');
  });

  it('shows the JT65 submode next to JT65 and asks for JT65A, JT65B or JT65C', () => {
    const onPick = vi.fn();
    const { unmount } = render(<ModeSwitch mode="JT65" onPick={onPick} />);
    const sub = screen.getByRole('combobox') as HTMLSelectElement;
    expect(sub.value).toBe('A');
    fireEvent.change(sub, { target: { value: 'B' } });
    expect(onPick).toHaveBeenLastCalledWith('JT65B');
    unmount();
    render(<ModeSwitch mode="JT65C" onPick={onPick} />);
    expect((screen.getByRole('combobox') as HTMLSelectElement).value).toBe('C');
    expect(screen.getByRole('button', { expanded: false })).toHaveAttribute('data-mode', 'JT65');
    openMenu();
    expect(screen.getByRole('menuitemradio', { name: 'JT65' })).toHaveAttribute('aria-checked', 'true');
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
