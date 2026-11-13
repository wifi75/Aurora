import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DigiNotch } from '../../api/client';
import { addNotch, moveNotch, notchAt, NOTCH_WIDTH_DEFAULT, NOTCH_WIDTH_MAX, NOTCH_WIDTH_MIN, removeNotch, resizeNotch } from '../../lib/digiNotches';
import { DigiWaterfall } from './DigiWaterfall';

beforeEach(() => {
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      disconnect() {}
    },
  );
  // jsdom non disegna: la cascata deve funzionare lo stesso
  HTMLCanvasElement.prototype.getContext = (() => null) as unknown as HTMLCanvasElement['getContext'];
});

function waterfall(notches: DigiNotch[], onNotches: (l: DigiNotch[]) => void, onPick = vi.fn()) {
  const view = render(<DigiWaterfall minHz={0} maxHz={4000} rxHz={1000} txHz={null} bandwidthHz={0} transmitting={false} onPick={onPick} notches={notches} onNotches={onNotches} />);
  const flow = view.container.querySelector('canvas.digi-wf-flow') as HTMLCanvasElement;
  // 800 punti per 4000 Hz: 5 Hz a punto
  flow.getBoundingClientRect = () => ({ left: 0, top: 0, width: 800, height: 100, right: 800, bottom: 100, x: 0, y: 0, toJSON: () => ({}) });
  return { ...view, flow };
}

describe('notch: i conti', () => {
  it('si aggiungono con la larghezza di base, non due nello stesso punto', () => {
    let l = addNotch([], 1500);
    expect(l).toEqual([{ hz: 1500, width_hz: NOTCH_WIDTH_DEFAULT }]);
    l = addNotch(l, 1505);
    expect(l).toHaveLength(1);
    l = addNotch(l, 1600);
    expect(l).toHaveLength(2);
    expect(notchAt(l, 1610)).toBe(1);
    expect(removeNotch(l, 0)).toEqual([{ hz: 1600, width_hz: NOTCH_WIDTH_DEFAULT }]);
  });

  it('si spostano e si allargano entro i limiti', () => {
    const l = [{ hz: 1000, width_hz: 30 }];
    expect(moveNotch(l, 0, 1200)[0]?.hz).toBe(1200);
    expect(resizeNotch(l, 0, 1020)[0]?.width_hz).toBe(40);
    expect(resizeNotch(l, 0, 1001)[0]?.width_hz).toBe(NOTCH_WIDTH_MIN);
    expect(resizeNotch(l, 0, 1500)[0]?.width_hz).toBe(NOTCH_WIDTH_MAX);
  });
});

describe('notch sulla cascata', () => {
  it('Ctrl+clic aggiunge un notch nel punto senza spostare la ricezione', () => {
    const onNotches = vi.fn();
    const onPick = vi.fn();
    const { flow } = waterfall([], onNotches, onPick);
    fireEvent.pointerDown(flow, { clientX: 300, ctrlKey: true, button: 0, pointerId: 1 });
    expect(onNotches).toHaveBeenCalledWith([{ hz: 1500, width_hz: NOTCH_WIDTH_DEFAULT }]);
    expect(onPick).not.toHaveBeenCalled();
  });

  it('il clic senza Ctrl sintonizza come prima', () => {
    const onNotches = vi.fn();
    const onPick = vi.fn();
    const { flow } = waterfall([], onNotches, onPick);
    fireEvent.pointerDown(flow, { clientX: 300, button: 0, pointerId: 1 });
    expect(onPick).toHaveBeenCalledWith(1500, false, false);
    expect(onNotches).not.toHaveBeenCalled();
  });

  it('i notch si vedono come strisce e il tasto destro li toglie', () => {
    const onNotches = vi.fn();
    waterfall([{ hz: 1500, width_hz: 30 }, { hz: 2000, width_hz: 50 }], onNotches);
    const strips = screen.getAllByTestId('digi-notch');
    expect(strips).toHaveLength(2);
    expect(strips[0]?.getAttribute('title')).toContain('1500');
    fireEvent.contextMenu(strips[0] as HTMLElement);
    expect(onNotches).toHaveBeenCalledWith([{ hz: 2000, width_hz: 50 }]);
  });
});
