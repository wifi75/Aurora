import { act, render, screen } from '@testing-library/react';
import type { ReactElement } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ ApiError: class ApiError extends Error {}, getControls: vi.fn().mockResolvedValue({}), postControl: vi.fn(), getOutputLevel: vi.fn().mockResolvedValue({ attenuation_db: 0, min_db: 0, max_db: 45 }), putOutputLevel: vi.fn(), getInputLevel: vi.fn().mockRejectedValue(new Error('no')), putInputLevel: vi.fn(), postTxMessage: vi.fn() }));
import { txAlertTone, visibleTxAlerts } from '../lib/txAlerts';
import { useStation } from '../state/station';
import { TxAlertBar } from './TxAlertBar';
import { TxPanel } from './TxPanel';

const withClient = (ui: ReactElement) => <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>;
const setAlerts = (tx_alerts: unknown[]) =>
  useStation.getState().apply({ type: 'status', dial_hz: 14_074_000, mode: 'FT8', tx_enabled: true, transmitting: false, decoding: false, de_call: 'IU3ABC', de_grid: 'IO91wm', tx_alerts } as never);

const swr = { kind: 'high_swr', value: 5, limit: 2.5 };
const stopped = { kind: 'tx_stopped', detail: 'watchdog|' };
const hound = { kind: 'hound_off', detail: 'K1ABC' };
const stuck = { kind: 'ptt_stuck', detail: 'x' };
const abandoned = { kind: 'ptt_abandoned' };

beforeEach(() => useStation.getState().reset());

describe('visibleTxAlerts', () => {
  it('drops the PTT alerts, which have their own banner', () => {
    expect(visibleTxAlerts([stuck, abandoned, hound] as never).map((a) => a.kind)).toEqual(['hound_off']);
    expect(visibleTxAlerts(undefined)).toEqual([]);
  });
  it('puts the red ones first and keeps the engine order inside each group', () => {
    const inhibited = { kind: 'inhibited', detail: 'x' };
    expect(visibleTxAlerts([hound, swr, inhibited, stopped] as never).map((a) => a.kind)).toEqual(['high_swr', 'tx_stopped', 'hound_off', 'inhibited']);
  });
  it('is red for high SWR and stop, amber for the others', () => {
    expect(txAlertTone(swr as never)).toBe('danger');
    expect(txAlertTone(stopped as never)).toBe('danger');
    expect(txAlertTone(hound as never)).toBe('warn');
  });
});

describe('TxAlertBar', () => {
  it('renders nothing without alerts or with only PTT ones', () => {
    setAlerts([stuck, abandoned]);
    const { container } = render(<TxAlertBar />);
    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByRole('alert')).toBeNull();
  });
  it('shows one row per alert, stacked, with the text of the alert', () => {
    setAlerts([hound, swr]);
    render(<TxAlertBar />);
    const bar = screen.getByRole('alert');
    const rows = bar.querySelectorAll('.tx-alert-bar-row');
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveClass('tx-alert-bar-row--danger');
    expect(rows[0]?.textContent).toContain('5.0');
    expect(rows[1]).toHaveClass('tx-alert-bar-row--warn');
  });
  it('goes away when the engine removes the alert', () => {
    setAlerts([swr]);
    render(<TxAlertBar />);
    expect(screen.getByRole('alert')).toBeInTheDocument();
    act(() => setAlerts([]));
    expect(screen.queryByRole('alert')).toBeNull();
  });
});

describe('TxPanel alerts placement', () => {
  it('keeps the alert in the panel by default (detached window)', () => {
    setAlerts([swr]);
    render(withClient(<TxPanel txAllowed part="combo" />));
    expect(document.querySelector('.tx-alerts')).not.toBeNull();
  });
  it('does not repeat it in the panel when the bar shows it', () => {
    setAlerts([swr]);
    render(withClient(<TxPanel txAllowed part="combo" alerts="bar" />));
    expect(document.querySelector('.tx-alerts')).toBeNull();
  });
});
