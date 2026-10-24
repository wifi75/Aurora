import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { BandBar } from './BandBar';

describe('BandBar', () => {
  it('lists the bands like JTDX and lights the current one', () => {
    render(<BandBar dialHz={21_074_000} />);
    const names = screen.getAllByRole('button').map((b) => b.textContent);
    expect(names).toEqual(['160', '80', '60', '40', '30', '20', '17', '15', '12', '10', '6', '2']);
    expect(screen.getByRole('button', { name: '15' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: '20' })).toHaveAttribute('aria-pressed', 'false');
  });

  it('lights nothing outside the bands', () => {
    render(<BandBar dialHz={0} />);
    expect(screen.getAllByRole('button').every((b) => b.getAttribute('aria-pressed') === 'false')).toBe(true);
  });

  it('only shows the band until the radio command is available', () => {
    render(<BandBar dialHz={14_074_000} />);
    expect(screen.getAllByRole('button').every((b) => (b as HTMLButtonElement).disabled)).toBe(true);
  });
});
