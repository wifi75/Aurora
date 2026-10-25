import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render as plainRender, screen } from '@testing-library/react';
import type { ReactElement } from 'react';
import { describe, expect, it } from 'vitest';
import type { StationStatus } from '../api/schemas';
import { useStation } from '../state/station';
import { DxPill } from './DxPill';

function render(ui: ReactElement) {
  return plainRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

const base: StationStatus = {
  dial_hz: 21_074_000, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: true,
  de_call: 'AA1AA', de_grid: 'FN42',
};

describe('DxPill', () => {
  it('puts the station being called in evidence', () => {
    render(<DxPill status={{ ...base, dx_call: 'BB2BB', dx_grid: 'KK00' }} />);
    const dx = screen.getByText('BB2BB');
    expect(dx.closest('.pill--dx')).not.toBeNull();
    expect(screen.getByText('KK00')).toBeInTheDocument();
  });

  it('shows flag and country from the messages of that station', () => {
    useStation.setState({ decodes: [{ id: 1, time_ms: 0, snr: -5, delta_time: 0.1, freq_hz: 1000, text: 'CQ DO1ID JO41', tag: null, country: 'Fed. Rep. of Germany' }] });
    const { container } = render(<DxPill status={{ ...base, dx_call: 'DO1ID', dx_grid: 'JO41' }} />);
    expect(screen.getByText('Fed. Rep. of Germany')).toBeInTheDocument();
    expect(container.querySelector('.pill--dx .fi-de')).not.toBeNull();
    useStation.setState({ decodes: [] });
  });

  it('shows nothing when nobody is being called', () => {
    const { container } = render(<DxPill status={base} />);
    expect(container.querySelector('.pill--dx')).toBeNull();
  });
});
