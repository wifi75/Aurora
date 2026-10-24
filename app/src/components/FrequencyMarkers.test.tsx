import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { FrequencyMarkers } from './FrequencyMarkers';

function rect(el: HTMLElement) {
  el.getBoundingClientRect = () => ({ left: 0, top: 0, right: 600, bottom: 300, width: 600, height: 300, x: 0, y: 0, toJSON: () => ({}) });
}

describe('FrequencyMarkers', () => {
  it('shows the green Rx and the red Tx marker at their frequencies', () => {
    render(<FrequencyMarkers fMinHz={0} fMaxHz={3000} rxHz={1000} txHz={1500} onRx={() => {}} onTx={() => {}} />);
    expect(screen.getByRole('slider', { name: /frequenza rx/i })).toHaveStyle({ left: '33.33333333333333%' });
    expect(screen.getByRole('slider', { name: /frequenza tx/i })).toHaveStyle({ left: '50%' });
    expect(screen.getByRole('slider', { name: /frequenza rx/i })).toHaveAttribute('aria-valuenow', '1000');
    expect(screen.getByText('Rx 1000')).toBeInTheDocument();
    expect(screen.getByText('Tx 1500')).toBeInTheDocument();
  });

  it('draws the bars as wide as the signal of the mode, from the frequency to the right', () => {
    const { rerender } = render(<FrequencyMarkers fMinHz={0} fMaxHz={3000} rxHz={1000} txHz={1500} bandwidthHz={50} onRx={() => {}} onTx={() => {}} />);
    const rx = screen.getByRole('slider', { name: /frequenza rx/i });
    expect(rx).toHaveClass('wf-marker--band');
    expect(rx).toHaveStyle({ left: '33.33333333333333%', width: '1.6666666666666667%' });
    rerender(<FrequencyMarkers fMinHz={0} fMaxHz={3000} rxHz={1000} txHz={1500} bandwidthHz={83.3} onRx={() => {}} onTx={() => {}} />);
    expect(screen.getByRole('slider', { name: /frequenza tx/i })).toHaveStyle({ width: '2.7766666666666664%' });
  });

  it('keeps the thin bar without a known bandwidth', () => {
    render(<FrequencyMarkers fMinHz={0} fMaxHz={3000} rxHz={1000} txHz={1500} onRx={() => {}} onTx={() => {}} />);
    expect(screen.getByRole('slider', { name: /frequenza rx/i })).not.toHaveClass('wf-marker--band');
  });

  it('shows nothing for a frequency the engine has not reported', () => {
    render(<FrequencyMarkers fMinHz={0} fMaxHz={3000} onRx={() => {}} onTx={() => {}} />);
    expect(screen.queryByRole('slider')).not.toBeInTheDocument();
  });

  it('a click on the spectrum sets Rx, shift+click sets Tx', () => {
    const onRx = vi.fn();
    const onTx = vi.fn();
    const { container } = render(<FrequencyMarkers fMinHz={0} fMaxHz={3000} rxHz={1000} txHz={1500} onRx={onRx} onTx={onTx} />);
    const pad = container.querySelector('.wf-pad') as HTMLElement;
    rect(pad.parentElement as HTMLElement);
    fireEvent.pointerDown(pad, { clientX: 300, clientY: 100, button: 0 });
    expect(onRx).toHaveBeenCalledWith(1500);
    fireEvent.pointerDown(pad, { clientX: 150, clientY: 100, button: 0, shiftKey: true });
    expect(onTx).toHaveBeenCalledWith(750);
  });

  it('moves with the keyboard in steps of 10 Hz (50 with shift)', () => {
    const onRx = vi.fn();
    render(<FrequencyMarkers fMinHz={0} fMaxHz={3000} rxHz={1000} txHz={1500} onRx={onRx} onTx={() => {}} />);
    const rx = screen.getByRole('slider', { name: /frequenza rx/i });
    fireEvent.keyDown(rx, { key: 'ArrowRight' });
    expect(onRx).toHaveBeenLastCalledWith(1010);
    fireEvent.keyDown(rx, { key: 'ArrowLeft', shiftKey: true });
    expect(onRx).toHaveBeenLastCalledWith(950);
  });
});
