import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { AlertSwitch } from './AlertSwitch';

describe('AlertSwitch', () => {
  it('shows whether the alerts are on and toggles them', () => {
    const onChange = vi.fn();
    render(<AlertSwitch value={{ enabled: false, volume: 0.5, sound: 'chime', muteWhileTx: true }} onChange={onChange} />);
    const button = screen.getByRole('button', { name: /avvisi sonori/i });
    expect(button).toHaveAttribute('aria-pressed', 'false');
    fireEvent.click(button);
    expect(onChange).toHaveBeenCalledWith({ enabled: true, volume: 0.5, sound: 'chime', muteWhileTx: true });
  });

  it('changes the volume only while on', () => {
    const onChange = vi.fn();
    const { rerender } = render(<AlertSwitch value={{ enabled: false, volume: 0.5, sound: 'chime', muteWhileTx: true }} onChange={onChange} />);
    expect(screen.getByLabelText(/volume/i)).toBeDisabled();
    rerender(<AlertSwitch value={{ enabled: true, volume: 0.5, sound: 'chime', muteWhileTx: true }} onChange={onChange} />);
    fireEvent.change(screen.getByLabelText(/volume/i), { target: { value: '0.9' } });
    expect(onChange).toHaveBeenCalledWith({ enabled: true, volume: 0.9, sound: 'chime', muteWhileTx: true });
  });
});
