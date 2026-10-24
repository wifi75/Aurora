import { fireEvent, render } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ postHalt: vi.fn().mockResolvedValue(undefined), postControl: vi.fn().mockResolvedValue(undefined) }));
import { postControl, postHalt } from '../api/client';
import { usePrefs } from './prefs';
import { useEscapeStop } from './escapeStop';

function Probe() {
  useEscapeStop();
  return null;
}

beforeEach(() => {
  vi.mocked(postHalt).mockClear();
  vi.mocked(postControl).mockClear();
  usePrefs.getState().reset();
});

describe('useEscapeStop', () => {
  it('on Escape halts the transmission and switches off AutoTX and Enable Tx', () => {
    render(<Probe />);
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(postHalt).toHaveBeenCalledTimes(1);
    expect(postControl).toHaveBeenCalledWith('auto_tx', false);
    expect(postControl).toHaveBeenCalledWith('enable_tx', false);
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
    expect(postControl).toHaveBeenCalledWith('auto_tx', false);
  });
});
