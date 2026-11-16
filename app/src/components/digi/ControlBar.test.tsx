import { act, fireEvent, render } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ControlBar, HOLD_MS, neighborSlot } from './ControlBar';
import type { Macro } from '../../lib/macroLang';

const macros = new Map<string, Macro>([['F1', { name: 'F1', label: 'CQ', text: 'CQ CQ', color: null }]]);

describe('neighborSlot', () => {
  it('si muove nella griglia 4×12 e si ferma ai bordi', () => {
    expect(neighborSlot('F1', 'right')).toBe('F2');
    expect(neighborSlot('F1', 'down')).toBe('Ctrl-F1');
    expect(neighborSlot('Shift-F12', 'down')).toBe('Ctrl-Shift-F12');
    expect(neighborSlot('F1', 'left')).toBeNull();
    expect(neighborSlot('Ctrl-Shift-F3', 'down')).toBeNull();
  });
});

describe('ControlBar: clic tenuto per spostare', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  const setup = () => {
    const onRun = vi.fn();
    const onMove = vi.fn();
    const r = render(<ControlBar macros={macros} extended onSetExtended={() => {}} onRun={onRun} onEdit={() => {}} onMove={onMove} />);
    const key = (n: string) => r.container.querySelector<HTMLElement>(`[data-macro="${n}"]`)!;
    return { onRun, onMove, key };
  };

  it('un clic breve esegue la macro', () => {
    const { onRun, onMove, key } = setup();
    fireEvent.pointerDown(key('F1'), { button: 0, clientX: 0, clientY: 0 });
    fireEvent.pointerUp(key('F1'), { clientX: 0, clientY: 0 });
    fireEvent.click(key('F1'));
    expect(onRun).toHaveBeenCalledTimes(1);
    expect(onMove).not.toHaveBeenCalled();
  });

  it('dopo la soglia il tasto si accende, il rilascio senza muoversi non esegue', () => {
    const { onRun, onMove, key } = setup();
    fireEvent.pointerDown(key('F1'), { button: 0, clientX: 0, clientY: 0 });
    act(() => void vi.advanceTimersByTime(HOLD_MS + 10));
    expect(key('F1').className).toContain('digi-key--armed');
    fireEvent.pointerUp(key('F1'), { clientX: 0, clientY: 0 });
    fireEvent.click(key('F1'));
    expect(onRun).not.toHaveBeenCalled();
    expect(onMove).not.toHaveBeenCalled();
  });

  it('trascinato su un altro tasto lo sposta', () => {
    const { onRun, onMove, key } = setup();
    const target = key('Ctrl-F5');
    document.elementFromPoint = () => target;
    fireEvent.pointerDown(key('F1'), { button: 0, clientX: 0, clientY: 0 });
    act(() => void vi.advanceTimersByTime(HOLD_MS + 10));
    fireEvent.pointerMove(key('F1'), { clientX: 300, clientY: 40 });
    expect(target.className).toContain('digi-key--drop');
    fireEvent.pointerUp(key('F1'), { clientX: 300, clientY: 40 });
    fireEvent.click(key('F1'));
    expect(onMove).toHaveBeenCalledWith('F1', 'Ctrl-F5');
    expect(onRun).not.toHaveBeenCalled();
  });

  it('Alt+freccia sposta il tasto col fuoco', () => {
    const { onMove, key } = setup();
    fireEvent.keyDown(key('F1'), { key: 'ArrowRight', altKey: true });
    expect(onMove).toHaveBeenCalledWith('F1', 'F2');
  });
});
