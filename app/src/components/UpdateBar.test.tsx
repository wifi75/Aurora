import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getUpdate: vi.fn() }));
import { getUpdate, type UpdateStatus } from '../api/client';
import { useStation } from '../state/station';
import { UpdateBar } from './UpdateBar';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

const release = { tag: 'v0.10.0', version: '0.10.0', name: 'Aurora 0.10.0', notes: '', url: 'https://github.com/x/y/releases/tag/v0.10.0', published_at: '', prerelease: false };
const status = (available: UpdateStatus['available']): UpdateStatus => ({
  current: '0.9.0', enabled: true, repo: 'wifi75/Aurora', interval_hours: 6, prereleases: false, has_token: false, checked_at_ms: 1, available, latest: available, error: null,
});

beforeEach(() => {
  useStation.getState().reset();
  localStorage.clear();
  vi.mocked(getUpdate).mockReset();
});

describe('UpdateBar', () => {
  it('shows nothing when Aurora is up to date', async () => {
    vi.mocked(getUpdate).mockResolvedValue(status(null));
    render(<UpdateBar />);
    await new Promise((r) => setTimeout(r, 50));
    expect(screen.queryByRole('status')).toBeNull();
  });

  it('shows the new version with a link to the release', async () => {
    vi.mocked(getUpdate).mockResolvedValue(status(release));
    render(<UpdateBar />);
    expect(await screen.findByText(/Nuova versione di Aurora: 0\.10\.0 \(in uso 0\.9\.0\)/)).toBeInTheDocument();
    const link = screen.getByRole('link', { name: 'Note e download' });
    expect(link).toHaveAttribute('href', release.url);
    expect(link).toHaveAttribute('rel', expect.stringContaining('noopener'));
  });

  it('hides for this version after Later and comes back for the next one', async () => {
    vi.mocked(getUpdate).mockResolvedValue(status(release));
    const { unmount } = render(<UpdateBar />);
    fireEvent.click(await screen.findByRole('button', { name: 'Più tardi' }));
    expect(screen.queryByRole('status')).toBeNull();
    unmount();
    render(<UpdateBar />);
    await new Promise((r) => setTimeout(r, 50));
    expect(screen.queryByRole('status')).toBeNull();
    unmount();
    vi.mocked(getUpdate).mockResolvedValue(status({ ...release, version: '0.10.1', tag: 'v0.10.1' }));
    render(<UpdateBar />);
    expect(await screen.findByText(/0\.10\.1/)).toBeInTheDocument();
  });

  it('shows the notice sent by the core at once, before the page has checked', async () => {
    vi.mocked(getUpdate).mockResolvedValue(status(null));
    useStation.getState().apply({ type: 'update_available', version: '0.11.0', tag: 'v0.11.0', notes: '', url: 'https://example.test/r' });
    render(<UpdateBar />);
    expect(await screen.findByText(/0\.11\.0/)).toBeInTheDocument();
  });
});
