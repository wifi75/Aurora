import { act, render } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAutosave } from './useAutosave';

function Probe({ value, save }: { value: string; save: (v: string) => void }) {
  useAutosave(value, true, save, () => undefined, 600);
  return <input type="password" aria-label="password" />;
}

describe('useAutosave', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('salva dopo la pausa', () => {
    const save = vi.fn();
    render(<Probe value="abc" save={save} />);
    act(() => void vi.advanceTimersByTime(700));
    expect(save).toHaveBeenCalledWith('abc');
  });

  it('non salva finche il cursore e in una casella password, poi salva appena se ne esce', () => {
    const save = vi.fn();
    const { getByLabelText } = render(<Probe value="ab" save={save} />);
    const field = getByLabelText('password') as HTMLInputElement;
    act(() => field.focus());
    act(() => void vi.advanceTimersByTime(3000));
    expect(save).not.toHaveBeenCalled();
    act(() => field.blur());
    act(() => void vi.advanceTimersByTime(700));
    expect(save).toHaveBeenCalledTimes(1);
    expect(save).toHaveBeenCalledWith('ab');
  });
});
