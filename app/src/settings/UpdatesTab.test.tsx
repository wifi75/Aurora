import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getUpdate: vi.fn(), putUpdate: vi.fn().mockResolvedValue({}), checkUpdate: vi.fn(), getDiagnostics: vi.fn().mockResolvedValue({ crash_folder: false }), openCrashFolder: vi.fn().mockResolvedValue(undefined) }));
import { checkUpdate, getUpdate, putUpdate, type UpdateStatus } from '../api/client';
import { UpdatesTab, draftToUpdate } from './UpdatesTab';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

const base: UpdateStatus = {
  current: '0.9.0', enabled: true, repo: 'wifi75/Aurora', interval_hours: 6, prereleases: false, has_token: false, checked_at_ms: 0, available: null, latest: null, error: null,
};
const draft = { enabled: true, repo: 'wifi75/Aurora', hours: '6', prereleases: false, token: '', removeToken: false };

describe('draftToUpdate', () => {
  it('builds the settings and only sends a token when one is written or removed', () => {
    expect(draftToUpdate(draft)).toEqual({ enabled: true, repo: 'wifi75/Aurora', interval_hours: 6, prereleases: false });
    expect(draftToUpdate({ ...draft, token: ' ghp_x ' })?.token).toBe('ghp_x');
    expect(draftToUpdate({ ...draft, removeToken: true })?.token).toBe('');
  });

  it('refuses what the core would refuse', () => {
    for (const bad of [{ repo: 'senza-barra' }, { repo: 'a/b/c' }, { repo: 'a b/c' }, { hours: '0' }, { hours: '169' }, { hours: 'x' }, { token: 'con spazio' }]) {
      expect(draftToUpdate({ ...draft, ...bad }), JSON.stringify(bad)).toBeNull();
    }
  });
});

describe('UpdatesTab', () => {
  beforeEach(() => {
    vi.mocked(getUpdate).mockReset().mockResolvedValue(base);
    vi.mocked(putUpdate).mockClear();
    vi.mocked(checkUpdate).mockReset();
  });

  it('shows the version in use and saves a change by itself', async () => {
    render(<UpdatesTab />);
    expect(await screen.findByText('0.9.0')).toBeInTheDocument();
    fireEvent.change(screen.getByDisplayValue('6'), { target: { value: '12' } });
    await waitFor(() => expect(putUpdate).toHaveBeenCalledWith({ enabled: true, repo: 'wifi75/Aurora', interval_hours: 12, prereleases: false }), { timeout: 3000 });
  });

  it('shows only the settings an operator needs: no repository, test versions or token', async () => {
    vi.mocked(getUpdate).mockResolvedValue({ ...base, has_token: true });
    render(<UpdatesTab />);
    await screen.findByRole('button', { name: 'Controlla ora' });
    expect(screen.queryByDisplayValue('wifi75/Aurora')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Togli il token' })).not.toBeInTheDocument();
  });

  it('checks now and says when a newer version is out or why the check failed', async () => {
    const release = { tag: 'v0.10.0', version: '0.10.0', name: 'x', notes: '', url: 'https://example.test/r', published_at: '', prerelease: false };
    vi.mocked(checkUpdate).mockResolvedValueOnce({ ...base, checked_at_ms: 5, error: 'nessuna release trovata' });
    render(<UpdatesTab />);
    fireEvent.click(await screen.findByRole('button', { name: 'Controlla ora' }));
    expect(await screen.findByText(/Il controllo non è riuscito: nessuna release trovata/)).toBeInTheDocument();
    vi.mocked(getUpdate).mockResolvedValue({ ...base, checked_at_ms: 9, latest: release, available: release });
    vi.mocked(checkUpdate).mockResolvedValueOnce({ ...base, checked_at_ms: 9, latest: release, available: release });
    fireEvent.click(screen.getByRole('button', { name: 'Controlla ora' }));
    expect(await screen.findByText(/È uscita la versione 0\.10\.0/)).toBeInTheDocument();
  });

  it('says so when a value is not valid and does not save it', async () => {
    render(<UpdatesTab />);
    const hours = await screen.findByDisplayValue(String(base.interval_hours));
    fireEvent.change(hours, { target: { value: '0' } });
    expect(await screen.findByText(/Un valore non va/)).toBeInTheDocument();
    await new Promise((r) => setTimeout(r, 900));
    expect(putUpdate).not.toHaveBeenCalled();
  });

  // il pulsante della cartella dei crash non sta piu' qui: e' in Impostazioni > Assistenza (SupportBox)
});
