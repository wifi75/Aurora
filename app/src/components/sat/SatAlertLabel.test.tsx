import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { satLabelStore } from '../../lib/satLabel';
import { SatAlertLabel } from './SatAlertLabel';

const T0 = 1_800_000_000;

describe('SatAlertLabel', () => {
  beforeEach(() => {
    satLabelStore.reset();
    location.hash = '#/';
  });

  it('non mostra niente senza etichetta', () => {
    const { container } = render(<SatAlertLabel />);
    expect(container).toBeEmptyDOMElement();
  });

  it('mostra nome e conto alla rovescia mm:ss, poi il tempo alla LOS', () => {
    render(<SatAlertLabel />);
    act(() => satLabelStore.setLabel({ norad: 25544, los: T0, name: 'ISS', phase: 'soon', seconds: 272 }));
    expect(screen.getByRole('status').textContent).toContain('ISS');
    expect(screen.getByRole('status').textContent).toContain('4:32');
    act(() => satLabelStore.setLabel({ norad: 25544, los: T0, name: 'ISS', phase: 'now', seconds: 540 }));
    expect(screen.getByRole('status').textContent).toContain('9:00');
    expect(screen.getByRole('status').textContent).toContain('LOS');
  });

  it('la X chiude l avviso di quel passaggio', () => {
    render(<SatAlertLabel />);
    act(() => satLabelStore.setLabel({ norad: 25544, los: T0, name: 'ISS', phase: 'soon', seconds: 60 }));
    fireEvent.click(screen.getByRole('button'));
    expect(screen.queryByRole('status')).toBeNull();
    expect(satLabelStore.get().dismissed).toEqual([{ norad: 25544, los: T0 }]);
  });

  it('il clic sull etichetta apre la pagina Satelliti', () => {
    render(<SatAlertLabel />);
    act(() => satLabelStore.setLabel({ norad: 25544, los: T0, name: 'ISS', phase: 'soon', seconds: 60 }));
    fireEvent.click(screen.getByRole('link'));
    expect(location.hash).toBe('#/satelliti');
  });
});
