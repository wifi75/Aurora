import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({
  postControlValue: vi.fn().mockResolvedValue(undefined),
}));
import { postControlValue } from '../api/client';
import { useControlValues } from '../state/controlValues';
import { WsprExtras, isBeaconMode } from './WsprExtras';

beforeEach(() => {
  vi.mocked(postControlValue).mockClear();
  useControlValues.setState({ values: { wspr_dbm: 37, wspr_pct: 20, wspr_type1: 1 } });
});

describe('WsprExtras', () => {
  it('shows up for WSPR and FST4W, with or without the period', () => {
    for (const m of ['WSPR', 'wspr', 'FST4W', 'FST4W-120', 'FST4W-1800']) expect(isBeaconMode(m), m).toBe(true);
    for (const m of ['FST4', 'FST4-60', 'FT8', 'JT65', '']) expect(isBeaconMode(m), m).toBe(false);
  });

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

  it('sends Tx Pct only on Enter or leaving the field, between 0 and 100, and toggles type 1 messages', async () => {
    render(<WsprExtras />);
    const pct = screen.getByRole('spinbutton');
    expect(pct).toHaveValue(20);
    // mentre si scrive non parte niente e il campo tiene i tasti
    fireEvent.change(pct, { target: { value: '33' } });
    expect(pct).toHaveValue(33);
    expect(postControlValue).not.toHaveBeenCalled();
    fireEvent.keyDown(pct, { key: 'Enter' });
    await waitFor(() => expect(postControlValue).toHaveBeenCalledWith('wspr_pct', 33));
    // vuoto: non manda 0 (il radiofaro non si ferma) e torna il valore del motore
    vi.mocked(postControlValue).mockClear();
    fireEvent.change(pct, { target: { value: '' } });
    fireEvent.blur(pct);
    expect(postControlValue).not.toHaveBeenCalled();
    expect(pct).toHaveValue(20);
    // oltre 100 si ferma a 100
    fireEvent.change(pct, { target: { value: '150' } });
    fireEvent.blur(pct);
    expect(postControlValue).not.toHaveBeenCalledWith('wspr_pct', 150);
    await waitFor(() => expect(postControlValue).toHaveBeenCalledWith('wspr_pct', 100));
    const type1 = screen.getByRole('button');
    expect(type1).toHaveAttribute('aria-pressed', 'true');
    fireEvent.click(type1);
    await waitFor(() => expect(postControlValue).toHaveBeenCalledWith('wspr_type1', 0));
  });
});
