import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({
  getDxcc: vi.fn().mockResolvedValue({ country: 'Solomon Islands', continent: 'OC', prefix: 'H4', cq_zone: 28, itu_zone: 51, lat: -9.43, lon: 159.95 }),
  getClusterSpots: vi.fn().mockResolvedValue({ config: {}, servers: [], spots: [] }),
  postFrequency: vi.fn(),
}));
import { DxpedDetail } from './DxpedDetail';
import { useStation } from '../state/station';

describe('DxpedDetail', () => {
  it('opens the card with the map and closes with Esc', async () => {
    useStation.setState({ status: { de_grid: 'IO91wm' } as never, decodes: [] });
    const onClose = vi.fn();
    render(
      <QueryClientProvider client={new QueryClient()}>
        <DxpedDetail d={{ call: 'H44MS', country: 'Solomon Is', start: '2026-10-03', end: '2026-11-20', qsl: 'LoTW', info: 'By DL2GAC', state: 'active' }} onClose={onClose} />
      </QueryClientProvider>,
    );
    expect(screen.getByRole('dialog', { name: 'H44MS' })).toBeInTheDocument();
    await waitFor(() => expect(document.querySelector('.dxd-map-big')).not.toBeNull());
    expect(onClose).not.toHaveBeenCalled();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).toHaveBeenCalled();
  });
});
