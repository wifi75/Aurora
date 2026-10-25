import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render as plainRender, screen } from '@testing-library/react';
import type { ReactElement } from 'react';
import { describe, expect, it } from 'vitest';
import type { StationStatus } from '../api/schemas';
import { StatusBar } from './StatusBar';

/** La barra di stato interroga l'API dell'orologio: serve un QueryClient (le chiamate falliscono e l'indicatore sta nascosto). */
function render(ui: ReactElement) {
  return plainRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

const base: StationStatus = {
  dial_hz: 21_074_000, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: true,
  de_call: 'AA1AA', de_grid: 'FN42',
};

describe('StatusBar', () => {
  it('does not show the DX pill any more: it lives in the Transmit panel', () => {
    const { container } = render(<StatusBar status={{ ...base, dx_call: 'BB2BB' }} connection="open" />);
    expect(container.querySelector('.pill--dx')).toBeNull();
  });

  it('keeps only the link and the frequency: band, mode, state and own station are shown elsewhere', () => {
    render(<StatusBar status={base} connection="open" />);
    expect(screen.getByText('21.074,0 kHz')).toBeInTheDocument();
    for (const gone of ['15m', 'FT8', 'AA1AA FN42']) expect(screen.queryByText(gone)).toBeNull();
  });
});
