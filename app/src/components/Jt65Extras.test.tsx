import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({
  getEngineSettings: vi.fn().mockRejectedValue(new Error('assenti')),
  putEngineSettings: vi.fn(),
  postControl: vi.fn().mockResolvedValue(undefined),
}));
import { postControl } from '../api/client';
import { translate } from '../i18n';
import { Jt65Extras } from './Jt65Extras';

const it_ = (key: Parameters<typeof translate>[1]) => translate('it', key);

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  vi.mocked(postControl).mockClear();
});

describe('Jt65Extras', () => {
  it('«Clear Avg» empties the JT65 average, like the WSJT-X button', async () => {
    render(<Jt65Extras />);
    fireEvent.click(screen.getByRole('button', { name: it_('mode.jt65clearAvg') }));
    await waitFor(() => expect(postControl).toHaveBeenCalledWith('jt65_clear_avg'));
  });

  it('is off while transmitting', () => {
    render(<Jt65Extras disabled />);
    expect(screen.getByRole('button', { name: it_('mode.jt65clearAvg') })).toBeDisabled();
  });
});
