import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getCallsign: vi.fn() }));
import { getCallsign } from '../api/client';
import { CallsignTip, TIP_DELAY_MS, clearCallsignCache } from './CallsignTip';

beforeEach(() => {
  vi.useFakeTimers();
  clearCallsignCache();
  vi.mocked(getCallsign).mockReset();
});
afterEach(() => vi.useRealTimers());

const info = {
  call: 'IU3ABC',
  dxcc: { country: 'Italy', continent: 'EU', prefix: 'I', cq_zone: 15, itu_zone: 28 },
  person: { fname: 'Mario', name: 'Rossi', city: 'Padova', country: 'Italy', source: 'QRZ.com' },
  lookup: 'found' as const,
  subscriber: true,
  message: null,
};

describe('CallsignTip', () => {
  it('waits a moment, shows flag, country, name and city, and asks only once', async () => {
    vi.mocked(getCallsign).mockResolvedValue(info);
    render(<CallsignTip call="IU3ABC">IU3ABC</CallsignTip>);
    const anchor = screen.getByText('IU3ABC');
    fireEvent.mouseEnter(anchor);
    expect(getCallsign).not.toHaveBeenCalled();
    await act(async () => {
      vi.advanceTimersByTime(TIP_DELAY_MS + 10);
    });
    const tip = screen.getByTestId('calltip');
    expect(tip).toHaveTextContent('Italy');
    expect(tip).toHaveTextContent('Mario Rossi');
    expect(tip).toHaveTextContent('Padova');
    expect(tip.querySelector('.fi-it')).not.toBeNull();
    fireEvent.mouseLeave(anchor);
    expect(screen.queryByTestId('calltip')).toBeNull();
    fireEvent.mouseEnter(anchor);
    await act(async () => {
      vi.advanceTimersByTime(TIP_DELAY_MS + 10);
    });
    expect(getCallsign).toHaveBeenCalledTimes(1);
  });

  it('passing quickly does not look anything up', async () => {
    render(<CallsignTip call="K1ABC">K1ABC</CallsignTip>);
    const anchor = screen.getByText('K1ABC');
    fireEvent.mouseEnter(anchor);
    fireEvent.mouseLeave(anchor);
    await act(async () => {
      vi.advanceTimersByTime(TIP_DELAY_MS * 2);
    });
    expect(getCallsign).not.toHaveBeenCalled();
  });

  it('without lookups shows only the country', async () => {
    vi.mocked(getCallsign).mockResolvedValue({ ...info, person: null, lookup: 'off', subscriber: null });
    render(<CallsignTip call="IU3ABC">IU3ABC</CallsignTip>);
    fireEvent.mouseEnter(screen.getByText('IU3ABC'));
    await act(async () => {
      vi.advanceTimersByTime(TIP_DELAY_MS + 10);
    });
    expect(screen.getByTestId('calltip')).toHaveTextContent('Italy');
    expect(screen.getByTestId('calltip')).not.toHaveTextContent('Mario');
  });
});
