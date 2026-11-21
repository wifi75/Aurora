import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../api/client', () => ({
  getWefaxCurrentBlob: vi.fn(() => Promise.reject(new Error('none'))),
  postWefaxSlant: vi.fn(),
}));
import { Viewer } from './Viewer';

beforeEach(() => {
  // jsdom non misura nulla: il riquadro del visore e' largo 904 px (la meta' della cartina IOC 576)
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(904);
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
  vi.stubGlobal('ResizeObserver', class { observe() {} unobserve() {} disconnect() {} });
});
afterEach(() => vi.restoreAllMocks());

function viewer() {
  return render(<Viewer snapRows={0} listening receiving={false} onSaved={() => undefined} onChanged={() => undefined} />);
}

describe('Viewer WEFAX', () => {
  it('parte adattato alla larghezza e senza barra orizzontale', () => {
    viewer();
    expect(screen.getByRole('status', { hidden: true }).textContent).toBe('50 %');
    expect(screen.getByTestId('wefax-box').className).toContain('wefax-box--fit');
    expect(screen.getByRole('button', { name: /fit|adatta/i, pressed: true })).toBeTruthy();
  });

  it('con + e − passa a una scala numerica dalla scala di adesso, Adatta torna indietro', () => {
    viewer();
    const box = screen.getByTestId('wefax-box');
    fireEvent.click(screen.getByLabelText(/zoom in|ingrandisci/i));
    expect(screen.getByRole('status', { hidden: true }).textContent).toBe('50 %');
    expect(box.className).not.toContain('wefax-box--fit');
    fireEvent.click(screen.getByLabelText(/zoom out|riduci/i));
    expect(screen.getByRole('status', { hidden: true }).textContent).toBe('33 %');
    fireEvent.click(screen.getByLabelText(/zoom in|ingrandisci/i));
    fireEvent.click(screen.getByRole('button', { name: /fit|adatta/i }));
    expect(box.className).toContain('wefax-box--fit');
    expect(screen.getByRole('status', { hidden: true }).textContent).toBe('50 %');
  });
});
