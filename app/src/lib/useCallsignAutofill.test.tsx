import { act, cleanup, renderHook } from '@testing-library/react';
import { useState } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getCallsign: vi.fn() }));
import { getCallsign, type CallsignInfo } from '../api/client';
import { clearCallsignCache } from '../components/CallsignTip';
import { AUTOFILL_DELAY_MS, useCallsignAutofill } from './useCallsignAutofill';

function info(call: string, fname: string, city: string, grid = ''): CallsignInfo {
  return { call, dxcc: null, person: { fname, name: '', city, country: '', grid, source: 'qrz' }, lookup: 'found', subscriber: null, message: null } as unknown as CallsignInfo;
}

function setup(enabled = true) {
  return renderHook(() => {
    const [f, setF] = useState({ call: '', name: '', qth: '', wwloc: '' });
    useCallsignAutofill(f.call, { name: f.name, qth: f.qth, wwloc: f.wwloc }, (p) => setF((x) => ({ ...x, ...p })), enabled);
    return { f, setF };
  });
}

async function wait() {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(AUTOFILL_DELAY_MS + 10);
  });
}

describe('useCallsignAutofill', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    clearCallsignCache();
    vi.mocked(getCallsign).mockReset();
  });
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it('precompila nome, QTH e locatore dopo l\'attesa', async () => {
    vi.mocked(getCallsign).mockResolvedValue(info('IU3ABC', 'Mario', 'Padova', 'jn55'));
    const { result } = setup();
    act(() => result.current.setF((x) => ({ ...x, call: 'IU3ABC' })));
    expect(getCallsign).not.toHaveBeenCalled();
    await wait();
    expect(result.current.f).toMatchObject({ name: 'Mario', qth: 'Padova', wwloc: 'JN55' });
  });

  it('non cerca un nominativo troppo corto', async () => {
    const { result } = setup();
    act(() => result.current.setF((x) => ({ ...x, call: 'IU' })));
    await wait();
    expect(getCallsign).not.toHaveBeenCalled();
  });

  it('non sovrascrive quello scritto a mano', async () => {
    vi.mocked(getCallsign).mockResolvedValue(info('IU3ABC', 'Mario', 'Padova'));
    const { result } = setup();
    act(() => result.current.setF((x) => ({ ...x, call: 'IU3ABC', name: 'Gigi' })));
    await wait();
    expect(result.current.f).toMatchObject({ name: 'Gigi', qth: 'Padova' });
  });

  it('cambiando nominativo sostituisce o svuota i valori automatici, non quelli a mano', async () => {
    vi.mocked(getCallsign).mockImplementation(async (c: string) => (c === 'IU3ABC' ? info(c, 'Mario', 'Padova') : info(c, 'Anna', '')));
    const { result } = setup();
    act(() => result.current.setF((x) => ({ ...x, call: 'IU3ABC' })));
    await wait();
    act(() => result.current.setF((x) => ({ ...x, call: 'IZ7EUB' })));
    expect(result.current.f).toMatchObject({ name: '', qth: '' });
    await wait();
    expect(result.current.f).toMatchObject({ name: 'Anna', qth: '' });
    act(() => result.current.setF((x) => ({ ...x, qth: 'Bari' })));
    act(() => result.current.setF((x) => ({ ...x, call: 'IU3ABC' })));
    await wait();
    expect(result.current.f).toMatchObject({ name: 'Mario', qth: 'Bari' });
  });

  it('senza ricerca configurata o con errore non cambia niente e non da errori', async () => {
    vi.mocked(getCallsign).mockResolvedValueOnce({ ...info('IU3ABC', '', ''), person: null, lookup: 'off' } as CallsignInfo).mockRejectedValue(new Error('giu'));
    const { result } = setup();
    act(() => result.current.setF((x) => ({ ...x, call: 'IU3ABC' })));
    await wait();
    act(() => result.current.setF((x) => ({ ...x, call: 'IZ7EUB' })));
    await wait();
    expect(result.current.f).toMatchObject({ name: '', qth: '', wwloc: '' });
  });

  it('disattivato non cerca', async () => {
    const { result } = setup(false);
    act(() => result.current.setF((x) => ({ ...x, call: 'IU3ABC' })));
    await wait();
    expect(getCallsign).not.toHaveBeenCalled();
  });
});
