import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({ getDxccMatrix: vi.fn() }));
import { DxccGrid, DxccLabelStrip } from '../components/DxccCard';
import { defaultDxccPrefs, inkFor, readDxccPrefs, saveDxccPrefs, useDxccPrefs } from '../lib/dxccPrefs';
import { sampleLabels, sampleMatrix, sampleStationCells } from '../lib/dxccSample';
import { DxccTab } from './DxccTab';

beforeEach(() => {
  window.localStorage.clear();
  useDxccPrefs.getState().reset();
});

function memory() {
  const data: Record<string, string> = {};
  return { data, getItem: (k: string) => data[k] ?? null, setItem: (k: string, v: string) => { data[k] = v; } };
}

describe('dxccPrefs', () => {
  it('start with every band, FT8 and FT4 (not DIG), factory colours and the full look', () => {
    const p = defaultDxccPrefs();
    expect(p.bands).toHaveLength(13);
    expect(p.rows).toEqual({ FT8: true, FT4: true, DIG: false });
    expect(p.cells).toEqual({ FT8: {}, FT4: {}, DIG: {} });
    expect([p.captions, p.total, p.tryField, p.compact]).toEqual([true, true, true, false]);
  });

  it('are saved and read back; wrong values fall back to the factory ones', () => {
    const store = memory();
    const p = defaultDxccPrefs();
    p.bands = ['20m', '40m'];
    p.cells.FT4.worked = '#112233';
    p.labels.new_one.onBg = '#AA0000';
    saveDxccPrefs(store, p);
    const back = readDxccPrefs(store);
    expect(back.bands).toEqual(['40m', '20m']);
    expect(back.cells.FT4.worked).toBe('#112233');
    expect(back.labels.new_one.onBg).toBe('#aa0000');
    store.data['aurora-dxcc-card'] = JSON.stringify({ bands: [], rows: { FT8: false, FT4: false, DIG: false }, cells: { FT8: { worked: 'red' } }, compact: 'si' });
    const bad = readDxccPrefs(store);
    expect(bad.bands).toHaveLength(13);
    expect(bad.rows.FT8).toBe(true);
    expect(bad.cells.FT8.worked).toBeUndefined();
    expect(bad.compact).toBe(false);
    store.data['aurora-dxcc-card'] = '{';
    expect(readDxccPrefs(store)).toEqual(defaultDxccPrefs());
  });

  it('pick a readable text colour on any background', () => {
    expect(inkFor('#ffffff')).toBe('#000000');
    expect(inkFor('#101010')).toBe('#ffffff');
  });
});

describe('the chosen colours reach the card', () => {
  it('colour the cells per mode: FT8 and FT4 can differ', () => {
    const p = defaultDxccPrefs();
    p.cells.FT8.worked = '#ff0000';
    p.cells.FT4.worked = '#0000ff';
    p.cells.FT4.unconfirmed = '#00ff00';
    useDxccPrefs.setState({ prefs: p });
    render(<DxccGrid matrix={sampleMatrix()} station={sampleStationCells()} />);
    const ft8 = within(screen.getByRole('row', { name: /FT8/ })).getAllByRole('cell');
    const ft4 = within(screen.getByRole('row', { name: /FT4/ })).getAllByRole('cell');
    const bands = sampleMatrix().bands;
    const at = (cells: HTMLElement[], band: string) => cells[bands.indexOf(band)]!;
    expect(at(ft8, '20m')).toHaveStyle({ backgroundColor: '#ff0000' });
    expect(at(ft4, '17m')).toHaveStyle({ backgroundColor: '#0000ff' });
    expect(at(ft4, '20m')).toHaveStyle({ backgroundColor: '#00ff00' });
    // le caselle senza scelta restano quelle del tema
    expect(at(ft8, '30m').getAttribute('style')).toBeNull();
    expect(at(ft4, '80m').getAttribute('style')).toBeNull();
  });

  it('colour each label when on and when off, background and text', () => {
    const p = defaultDxccPrefs();
    p.labels.new_band.onBg = '#123456';
    p.labels.new_band.onInk = '#fedcba';
    p.labels.new_one.offBg = '#222222';
    useDxccPrefs.setState({ prefs: p });
    render(<DxccLabelStrip labels={sampleLabels()} />);
    expect(screen.getByText('NEW BAND', { selector: 'li' })).toHaveStyle({ backgroundColor: '#123456', color: '#fedcba' });
    expect(screen.getByText('NEW ONE', { selector: 'li' })).toHaveStyle({ backgroundColor: '#222222' });
    expect(screen.getByText('NEW GRID', { selector: 'li' }).getAttribute('style')).toBeNull();
  });

  it('shows only the chosen bands and rows, and hides the total', () => {
    const p = defaultDxccPrefs();
    p.bands = ['20m', '40m'];
    p.rows = { FT8: true, FT4: false, DIG: true };
    p.total = false;
    useDxccPrefs.setState({ prefs: p });
    render(<DxccGrid matrix={sampleMatrix()} station={null} />);
    expect(screen.getAllByRole('columnheader')).toHaveLength(3);
    expect(screen.queryByRole('row', { name: /FT4/ })).toBeNull();
    expect(screen.getByRole('row', { name: /DIG/ })).toBeInTheDocument();
    expect(screen.queryByText('214')).toBeNull();
  });
});

describe('DxccTab', () => {
  function wrap() {
    return render(
      <QueryClientProvider client={new QueryClient()}>
        <DxccTab />
      </QueryClientProvider>,
    );
  }

  it('turns a band and a row on or off, keeping at least one band', async () => {
    const user = userEvent.setup();
    wrap();
    const checks = screen.getAllByRole('checkbox');
    const band20 = checks.find((c) => c.closest('label')?.textContent === '20m')!;
    await user.click(band20);
    expect(useDxccPrefs.getState().prefs.bands).not.toContain('20m');
    const dig = checks.find((c) => c.closest('label')?.textContent === 'DIG')!;
    await user.click(dig);
    expect(useDxccPrefs.getState().prefs.rows.DIG).toBe(true);
    expect(JSON.parse(window.localStorage.getItem('aurora-dxcc-card')!).rows.DIG).toBe(true);
    useDxccPrefs.getState().set({ bands: ['20m'] });
    await user.click(band20);
    expect(useDxccPrefs.getState().prefs.bands).toEqual(['20m']);
  });

  it('saves a colour chosen for one mode, shows it in the preview and restores the factory colours', async () => {
    const user = userEvent.setup();
    wrap();
    const picker = screen.getByLabelText('Colore della casella: lavorato, nel modo FT4.');
    fireEvent.input(picker, { target: { value: '#ab12cd' } });
    expect(useDxccPrefs.getState().prefs.cells.FT4.worked).toBe('#ab12cd');
    expect(useDxccPrefs.getState().prefs.cells.FT8.worked).toBeUndefined();
    const row = screen.getByRole('row', { name: /FT4/ });
    expect(within(row).getAllByRole('cell').some((c) => c.style.background !== '')).toBe(true);
    await user.click(screen.getByRole('button', { name: 'Ripristina i colori di fabbrica' }));
    expect(useDxccPrefs.getState().prefs.cells.FT4).toEqual({});
  });

  it('switches the look options', async () => {
    const user = userEvent.setup();
    wrap();
    await user.click(screen.getByRole('checkbox', { name: 'Dimensione compatta' }));
    await user.click(screen.getByRole('checkbox', { name: 'Mostra le didascalie sotto le etichette' }));
    const p = useDxccPrefs.getState().prefs;
    expect([p.compact, p.captions]).toEqual([true, false]);
    expect(document.querySelector('.dxcc-card--compact')).not.toBeNull();
  });
});
