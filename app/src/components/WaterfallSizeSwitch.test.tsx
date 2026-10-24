import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { WaterfallSizeSwitch } from './WaterfallSizeSwitch';

describe('WaterfallSizeSwitch', () => {
  it('offers the three sizes and marks the current one', () => {
    render(<WaterfallSizeSwitch value="small" onChange={() => {}} />);
    expect(screen.getByRole('button', { name: 'Grande' })).toHaveAttribute('aria-pressed', 'false');
    expect(screen.getByRole('button', { name: 'Ridotta' })).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByRole('button', { name: 'Nascosta' })).toHaveAttribute('aria-pressed', 'false');
  });

  it('reports the chosen size', () => {
    const onChange = vi.fn();
    render(<WaterfallSizeSwitch value="large" onChange={onChange} />);
    fireEvent.click(screen.getByRole('button', { name: 'Nascosta' }));
    expect(onChange).toHaveBeenCalledWith('hidden');
    fireEvent.click(screen.getByRole('button', { name: 'Ridotta' }));
    expect(onChange).toHaveBeenCalledWith('small');
  });

  it('is labelled as a group for assistive technologies', () => {
    render(<WaterfallSizeSwitch value="large" onChange={() => {}} />);
    expect(screen.getByRole('group', { name: 'Dimensione della cascata' })).toBeInTheDocument();
  });
});
