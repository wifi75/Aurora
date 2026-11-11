import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({
  postControlValue: vi.fn().mockResolvedValue(undefined),
}));
import { postControlValue } from '../api/client';
import { useControlValues } from '../state/controlValues';
import { WsprExtras } from './WsprExtras';

beforeEach(() => {
  vi.mocked(postControlValue).mockClear();
  useControlValues.setState({ values: { wspr_dbm: 37, wspr_pct: 20, wspr_type1: 1 } });
});

describe('WsprExtras', () => {
  it('offers the WSJT-X power list from 0 to 60 dBm and sends the chosen one', async () => {
    render(<WsprExtras />);
    const select = screen.getByRole('combobox');
    const values = Array.from(select.querySelectorAll('option')).map((o) => o.value);
    expect(values).toHaveLength(19);
    expect(values[0]).toBe('0');
    expect(values[18]).toBe('60');
    expect(select).toHaveValue('37');
    fireEvent.change(select, { target: { value: '30' } });
    await waitFor(() => expect(postControlValue).toHaveBeenCalledWith('wspr_dbm', 30));
  });

  it('sends Tx Pct between 0 and 100 and toggles type 1 messages', async () => {
    render(<WsprExtras />);
    const pct = screen.getByRole('spinbutton');
    expect(pct).toHaveValue(20);
    fireEvent.change(pct, { target: { value: '33' } });
    await waitFor(() => expect(postControlValue).toHaveBeenCalledWith('wspr_pct', 33));
    fireEvent.change(pct, { target: { value: '150' } });
    expect(postControlValue).not.toHaveBeenCalledWith('wspr_pct', 150);
    const type1 = screen.getByRole('button');
    expect(type1).toHaveAttribute('aria-pressed', 'true');
    fireEvent.click(type1);
    await waitFor(() => expect(postControlValue).toHaveBeenCalledWith('wspr_type1', 0));
  });
});
