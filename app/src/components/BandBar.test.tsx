import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { BandBar } from './BandBar';

describe('BandBar', () => {
  it('lists the bands like the old one and lights the current one', () => {
    render(<BandBar dialHz={21_074_000} />);
    const names = screen.getAllByRole('button').map((b) => b.textContent);
    expect(names).toEqual(['160', '80', '60', '40', '30', '20', '17', '15', '12', '10', '6', '2']);
    expect(screen.getByRole('button', { name: '15' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: '20' })).toHaveAttribute('aria-pressed', 'false');
  });

  it('shows only the chosen bands', () => {
    render(<BandBar dialHz={14_074_000} bands={['40m', '20m']} />);
    expect(screen.getAllByRole('button').map((b) => b.textContent)).toEqual(['40', '20']);
  });

  it('marks the current band so it can be told at a glance', () => {
    render(<BandBar dialHz={14_074_000} />);
    expect(screen.getByRole('button', { name: '20' })).toHaveAttribute('aria-current', 'true');
    expect(screen.getByRole('button', { name: '40' })).not.toHaveAttribute('aria-current');
  });

  it('lights nothing outside the bands', () => {
    render(<BandBar dialHz={0} />);
    expect(screen.getAllByRole('button').every((b) => b.getAttribute('aria-pressed') === 'false')).toBe(true);
  });

  it('only shows the band when there is no radio command', () => {
    render(<BandBar dialHz={14_074_000} />);
    expect(screen.getAllByRole('button').every((b) => (b as HTMLButtonElement).disabled)).toBe(true);
  });

  it('asks for the band that was clicked when the radio can be controlled', () => {
    const onPick = vi.fn();
    render(<BandBar dialHz={14_074_000} onPick={onPick} />);
    fireEvent.click(screen.getByRole('button', { name: '40' }));
    expect(onPick).toHaveBeenCalledWith('40m');
  });

  it('is locked while transmitting', () => {
    render(<BandBar dialHz={14_074_000} onPick={() => {}} locked />);
    expect(screen.getAllByRole('button').every((b) => (b as HTMLButtonElement).disabled)).toBe(true);
  });
});
