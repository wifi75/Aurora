import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getCallsign: vi.fn() }));
import { getCallsign } from '../api/client';
var myGrid = '';
vi.mock('../state/station', () => ({ useStatusPick: () => (myGrid ? { de_grid: myGrid } : null) }));
import { CallsignTip, TIP_DELAY_MS, callsignCacheSize, clearCallsignCache, knownCallsign, lookupCallsign } from './CallsignTip';

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

  it('a failed lookup is not kept: it is asked again a little later', async () => {
    vi.mocked(getCallsign).mockRejectedValueOnce(new Error('giu')).mockResolvedValue(info);
    expect(await lookupCallsign('IU3ABC')).toBeNull();
    expect(knownCallsign('IU3ABC')).toBeUndefined();
    // subito dopo non si ripete (niente raffica col precaricamento)
    await lookupCallsign('IU3ABC');
    expect(getCallsign).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(31_000);
    expect(await lookupCallsign('IU3ABC')).toEqual(info);
    expect(getCallsign).toHaveBeenCalledTimes(2);
  });

  it('an answer without the LoTW upload is refreshed after a few minutes, one with it is kept', async () => {
    vi.mocked(getCallsign).mockResolvedValueOnce(info).mockResolvedValue({ ...info, lotw_last_upload: '2026-11-01' });
    await lookupCallsign('IU3ABC');
    await lookupCallsign('IU3ABC');
    expect(getCallsign).toHaveBeenCalledTimes(1);
    vi.advanceTimersByTime(6 * 60_000);
    expect((await lookupCallsign('IU3ABC'))?.lotw_last_upload).toBe('2026-11-01');
    vi.advanceTimersByTime(60 * 60_000);
    await lookupCallsign('IU3ABC');
    expect(getCallsign).toHaveBeenCalledTimes(2);
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

describe('callsign cache', () => {
  it('remembers at most 3000 callsigns, forgetting the oldest ones', async () => {
    vi.mocked(getCallsign).mockImplementation((call: string) => Promise.resolve({ ...info, call }));
    for (let i = 0; i < 3100; i++) void lookupCallsign(`K${i}ABC`);
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(callsignCacheSize().cache).toBe(3000);
    expect(callsignCacheSize().ready).toBeLessThanOrEqual(3000);
    expect(knownCallsign('K0ABC')).toBeUndefined();
    expect(knownCallsign('K3099ABC')?.call).toBe('K3099ABC');
  });
});

describe('CallsignTip bearing', () => {
  afterEach(() => {
    myGrid = '';
  });
  it('shows azimuth and distance when both locators are known', async () => {
    myGrid = 'JN55';
    vi.mocked(getCallsign).mockResolvedValue(info);
    render(<CallsignTip call="IU3ABC" grid="JN65">IU3ABC</CallsignTip>);
    fireEvent.mouseEnter(screen.getByText('IU3ABC'));
    await act(async () => {
      vi.advanceTimersByTime(TIP_DELAY_MS + 10);
    });
    expect(screen.getByTestId('calltip')).toHaveTextContent(/Azimut \d+° · \d+ km/);
  });

  it('shows no bearing without our own locator', async () => {
    vi.mocked(getCallsign).mockResolvedValue(info);
    render(<CallsignTip call="IU3ABC" grid="JN65">IU3ABC</CallsignTip>);
    fireEvent.mouseEnter(screen.getByText('IU3ABC'));
    await act(async () => {
      vi.advanceTimersByTime(TIP_DELAY_MS + 10);
    });
    expect(screen.getByTestId('calltip')).not.toHaveTextContent('Azimut');
  });
});
