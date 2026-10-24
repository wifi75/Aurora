import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { StationStatus } from '../api/schemas';
import { useStation } from '../state/station';
import { StatusBar } from './StatusBar';

const base: StationStatus = {
  dial_hz: 21_074_000, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: true,
  de_call: 'AA1AA', de_grid: 'FN42',
};

describe('StatusBar', () => {
  it('puts the station being called in evidence', () => {
    render(<StatusBar status={{ ...base, dx_call: 'BB2BB', dx_grid: 'KK00' }} connection="open" />);
    const dx = screen.getByText('BB2BB');
    expect(dx).toBeInTheDocument();
    expect(dx.closest('.pill--dx')).not.toBeNull();
    expect(screen.getByText('KK00')).toBeInTheDocument();
  });

  it('shows flag and country small under the DX call, from the messages of that station', () => {
    useStation.setState({ decodes: [{ id: 1, time_ms: 0, snr: -5, delta_time: 0.1, freq_hz: 1000, text: 'CQ DO1ID JO41', tag: null, country: 'Fed. Rep. of Germany' }] });
    const { container } = render(<StatusBar status={{ ...base, dx_call: 'DO1ID', dx_grid: 'JO41' }} connection="open" />);
    expect(screen.getByText('Fed. Rep. of Germany')).toBeInTheDocument();
    expect(container.querySelector('.pill--dx .fi-de')).not.toBeNull();
    useStation.setState({ decodes: [] });
  });

  it('shows no DX pill when nobody is being called', () => {
    const { container } = render(<StatusBar status={base} connection="open" />);
    expect(container.querySelector('.pill--dx')).toBeNull();
  });

  it('still shows band, frequency, mode and own station', () => {
    render(<StatusBar status={base} connection="open" />);
    expect(screen.getByText('15m')).toBeInTheDocument();
    expect(screen.getByText('21.074 MHz')).toBeInTheDocument();
    expect(screen.getByText('FT8')).toBeInTheDocument();
    expect(screen.getByText('AA1AA FN42')).toBeInTheDocument();
  });
});
