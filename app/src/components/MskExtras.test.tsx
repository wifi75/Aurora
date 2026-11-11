import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({
  getControls: vi.fn(),
  postControl: vi.fn().mockResolvedValue(undefined),
  postControlValue: vi.fn().mockResolvedValue(undefined),
}));
import { getControls, postControl, postControlValue } from '../api/client';
import { useControlValues } from '../state/controlValues';
import { MskExtras } from './MskExtras';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  vi.mocked(postControl).mockClear();
  vi.mocked(postControlValue).mockClear();
  vi.mocked(getControls).mockReset().mockResolvedValue({ msk_sh: false, swl: true });
  useControlValues.setState({ values: { msk144_period: 30 } });
});

describe('MskExtras', () => {
  it('offers the WSJT-X T/R periods 5, 10, 15 and 30 s and sends the chosen one', async () => {
    render(<MskExtras />);
    const select = screen.getByRole('combobox');
    expect(Array.from(select.querySelectorAll('option')).map((o) => o.value)).toEqual(['5', '10', '15', '30']);
    expect(select).toHaveValue('30');
    fireEvent.change(select, { target: { value: '15' } });
    await waitFor(() => expect(postControlValue).toHaveBeenCalledWith('msk144_period', 15));
  });

  it('switches Sh on (off by default, like WSJT-X)', async () => {
    render(<MskExtras />);
    const sh = screen.getByRole('button', { name: 'Sh' });
    await waitFor(() => expect(sh).toHaveAttribute('aria-pressed', 'false'));
    fireEvent.click(sh);
    await waitFor(() => expect(postControl).toHaveBeenCalledWith('msk_sh', true));
  });
});
