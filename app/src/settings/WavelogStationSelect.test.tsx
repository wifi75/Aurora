import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ postWavelogStations: vi.fn() }));
import { postWavelogStations } from '../api/client';
import { WavelogStationSelect } from './WavelogStationSelect';

function wrap(ui: React.ReactElement) {
  return render(<QueryClientProvider client={new QueryClient()}>{ui}</QueryClientProvider>);
}

describe('WavelogStationSelect', () => {
  it('fills the list with the stations read from the server', async () => {
    vi.mocked(postWavelogStations).mockResolvedValue([
      { id: '1', name: 'Casa', call: 'K1ABC' },
      { id: '2', name: 'Portatile', call: 'K1ABC/P' },
    ]);
    const onChange = vi.fn();
    wrap(<WavelogStationSelect id="x" label="Stazione" url="https://log.example.com" value="1" disabled={false} onChange={onChange} />);
    expect(await screen.findByRole('option', { name: '2 — Portatile (K1ABC/P)' })).toBeInTheDocument();
    fireEvent.change(screen.getByRole('combobox', { name: 'Stazione' }), { target: { value: '2' } });
    expect(onChange).toHaveBeenCalledWith('2');
    expect(postWavelogStations).toHaveBeenCalledWith('https://log.example.com');
  });

  it('keeps the number box when the server cannot be read', async () => {
    vi.mocked(postWavelogStations).mockRejectedValue(new Error('no'));
    wrap(<WavelogStationSelect id="x" label="Stazione" url="https://log.example.com" value="3" disabled={false} onChange={() => {}} />);
    expect(await screen.findByText(/Stazioni non lette/)).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Stazione' })).toHaveValue('3');
  });
});
