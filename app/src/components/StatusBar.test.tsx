import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { StationStatus } from '../api/schemas';
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
