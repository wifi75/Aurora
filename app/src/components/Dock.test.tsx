import { act, fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { defaultLayout, type PanelId } from '../lib/layout';
import { Dock } from './Dock';

const titles: Record<PanelId, string> = { waterfall: 'Cascata', station: 'Stazione', tx: 'Trasmissione', cq: 'CQ', all: 'Tutti', log: 'Log', cluster: 'DX Cluster', dxcc: 'DXCC' };

function show(onChange = vi.fn()) {
  const view = render(<Dock layout={defaultLayout()} onChange={onChange} titles={titles} render={(id) => <p>contenuto {id}</p>} />);
  return { ...view, onChange };
}

const grip = (name: string) => screen.getByRole('heading', { name }).closest('header') as HTMLElement;

describe('Dock', () => {
  it('shows every panel of the layout', () => {
    show();
    for (const name of Object.values(titles)) expect(screen.getByRole('heading', { name })).toBeInTheDocument();
  });

  it('never hides a panel while it is being dragged', () => {
    show();
    fireEvent.pointerDown(grip('Trasmissione'), { button: 0, clientX: 10, clientY: 10 });
    expect(screen.getByRole('heading', { name: 'Trasmissione' })).toBeInTheDocument();
    expect(screen.getByText('contenuto tx')).toBeInTheDocument();
  });

  it('a pointer release anywhere ends the drag and does not leave a drop marker behind', () => {
    const { container } = show();
    fireEvent.pointerDown(grip('Trasmissione'), { button: 0, clientX: 10, clientY: 10 });
    expect(container.querySelector('.dock--dragging')).not.toBeNull();
    act(() => {
      window.dispatchEvent(new Event('pointerup'));
    });
    expect(container.querySelector('.dock--dragging')).toBeNull();
    expect(container.querySelector('.dock-drop')).toBeNull();
  });

  it('cancel, Escape and window blur also end the drag', () => {
    const { container } = show();
    for (const end of [() => window.dispatchEvent(new Event('pointercancel')), () => window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' })), () => window.dispatchEvent(new Event('blur'))]) {
      fireEvent.pointerDown(grip('CQ'), { button: 0, clientX: 10, clientY: 10 });
      expect(container.querySelector('.dock--dragging')).not.toBeNull();
      act(() => {
        end();
      });
      expect(container.querySelector('.dock--dragging')).toBeNull();
    }
  });

  it('double click on the title folds the panel', () => {
    const { onChange } = show();
    fireEvent.doubleClick(grip('Cascata'));
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ collapsed: ['waterfall'] }));
  });
});
