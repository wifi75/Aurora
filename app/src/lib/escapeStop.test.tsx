import { fireEvent, render } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ postHalt: vi.fn().mockResolvedValue(undefined), postControl: vi.fn().mockResolvedValue(undefined) }));
import { postControl, postHalt } from '../api/client';
import { usePrefs } from './prefs';
import { useStation } from '../state/station';
import { useEscapeStop } from './escapeStop';

function Probe() {
  useEscapeStop();
  return null;
}

beforeEach(() => {
  vi.useRealTimers();
  useStation.getState().setPendingDx(null);
  vi.mocked(postHalt).mockClear();
  vi.mocked(postControl).mockClear();
  usePrefs.getState().reset();
});

describe('useEscapeStop', () => {
  it('on Escape halts the transmission and switches Enable Tx off, like JTDX, and leaves AutoTX alone', () => {
    render(<Probe />);
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(postHalt).toHaveBeenCalledTimes(1);
    expect(postControl).toHaveBeenCalledWith('enable_tx', false);
    expect(postControl).not.toHaveBeenCalledWith('auto_tx', expect.anything());
  });

  it('ignores every other key', () => {
    render(<Probe />);
    fireEvent.keyDown(window, { key: 'Enter' });
    fireEvent.keyDown(window, { key: 'e' });
    expect(postHalt).not.toHaveBeenCalled();
  });

  it('does nothing when the option is off', () => {
    usePrefs.getState().set({ escStop: false });
    render(<Probe />);
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(postHalt).not.toHaveBeenCalled();
    expect(postControl).not.toHaveBeenCalled();
  });

  it('still switches the switches off when the halt call fails', async () => {
    vi.mocked(postHalt).mockRejectedValueOnce(new Error('down'));
    render(<Probe />);
    fireEvent.keyDown(window, { key: 'Escape' });
    await Promise.resolve();
    expect(postControl).toHaveBeenCalledWith('enable_tx', false);
  });

  it('a single Escape does not clear the station being called', () => {
    useStation.getState().setPendingDx('DO1ID');
    render(<Probe />);
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(postControl).not.toHaveBeenCalledWith('clear_dx');
    expect(useStation.getState().pendingDx).toBe('DO1ID');
  });

  it('a double Escape also clears the DX station and the one chosen by double click', () => {
    vi.useFakeTimers();
    useStation.getState().setPendingDx('DO1ID');
    render(<Probe />);
    fireEvent.keyDown(window, { key: 'Escape' });
    vi.advanceTimersByTime(300);
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(postControl).toHaveBeenCalledWith('clear_dx');
    expect(useStation.getState().pendingDx).toBeNull();
    expect(postHalt).toHaveBeenCalledTimes(2);
  });

  it('two Escapes too far apart are two single ones, and a third one starts over', () => {
    vi.useFakeTimers();
    render(<Probe />);
    fireEvent.keyDown(window, { key: 'Escape' });
    vi.advanceTimersByTime(1500);
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(postControl).not.toHaveBeenCalledWith('clear_dx');
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(postControl).toHaveBeenCalledWith('clear_dx');
    vi.mocked(postControl).mockClear();
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(postControl).not.toHaveBeenCalledWith('clear_dx');
  });
});
