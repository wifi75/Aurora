import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { StationStatus } from '../api/schemas';
import { StationPanel } from './StationPanel';

const status = (over: Partial<StationStatus> = {}): StationStatus => ({
  dial_hz: 21_074_000, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: true,
  de_call: 'IU3CYV', de_grid: 'JN55', ...over,
});

describe('StationPanel', () => {
  it('shows band, frequency, mode and station', () => {
    render(<StationPanel status={status()} />);
    expect(screen.getByText('15m')).toBeInTheDocument();
    expect(screen.getByText('21.074 MHz')).toBeInTheDocument();
    expect(screen.getByText('FT8')).toBeInTheDocument();
    expect(screen.getByText('IU3CYV JN55')).toBeInTheDocument();
  });

  it('says whether it is receiving', () => {
    const { rerender } = render(<StationPanel status={status({ decoding: true })} />);
    expect(screen.getByText('In ricezione')).toBeInTheDocument();
    rerender(<StationPanel status={status({ decoding: false })} />);
    expect(screen.getByText('In attesa del prossimo ciclo')).toBeInTheDocument();
  });

  it('shows the transmit state clearly', () => {
    const { rerender } = render(<StationPanel status={status()} />);
    expect(screen.getByText('Spenta')).toBeInTheDocument();
    rerender(<StationPanel status={status({ tx_enabled: true })} />);
    expect(screen.getByText('Abilitata')).toBeInTheDocument();
    rerender(<StationPanel status={status({ tx_enabled: true, transmitting: true })} />);
    expect(screen.getByText('In trasmissione')).toBeInTheDocument();
  });

  it('shows a waiting message before JTDX is known', () => {
    render(<StationPanel status={null} />);
    expect(screen.getByText('In attesa di JTDX…')).toBeInTheDocument();
  });

  it('shows a dash for a frequency outside the amateur bands', () => {
    render(<StationPanel status={status({ dial_hz: 0 })} />);
    expect(screen.getByText('Banda non nota')).toBeInTheDocument();
  });
});
