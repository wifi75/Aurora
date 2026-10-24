import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { COLOR_KEYS, COLOR_LABELS, defaultColors, readColors } from '../lib/colors';
import { defaultLayout, readLayout, resizePanel, saveLayout } from '../lib/layout';
vi.mock('../api/client', () => ({
  getStationInfo: vi.fn().mockResolvedValue({
    tx_allowed: false,
    export_udp: ['127.0.0.1:2239'],
    export_file: null,
    udp_listen: '127.0.0.1:2334',
    udp_forward: ['127.0.0.1:2333'],
    audio_device: 'USB Audio',
    cty_loaded: 346,
    worked_qsos: 2056,
    limits: { max_consecutive_cycles: 6, max_per_day: 40, dead_man_secs: 120, run_unattended: false },
  }),
}));
import { Settings } from './Settings';

beforeEach(() => {
  localStorage.clear();
  document.documentElement.removeAttribute('style');
});

function open() {
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <Settings />
    </QueryClientProvider>,
  );
}

describe('Settings', () => {
  it('has a separate tab for each kind of configuration and shows one at a time', () => {
    open();
    const names = screen.getAllByRole('tab').map((t) => t.textContent);
    expect(names).toEqual(['Colori', 'Finestre', 'Avvisi sonori', 'Trasmissione', 'Log esterni', 'Collegamenti']);
    expect(screen.getByRole('tab', { name: 'Colori' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('tabpanel')).toHaveAccessibleName('Colori');
    fireEvent.click(screen.getByRole('tab', { name: 'Finestre' }));
    expect(screen.getByRole('tabpanel')).toHaveAccessibleName('Finestre');
    expect(screen.queryByLabelText('Nuovo DXCC')).not.toBeInTheDocument();
  });

  it('shows the transmission state and says it is closed', async () => {
    open();
    fireEvent.click(screen.getByRole('tab', { name: 'Trasmissione' }));
    expect(await screen.findByText('Trasmissione chiusa')).toBeInTheDocument();
  });

  it('shows the external log targets', async () => {
    open();
    fireEvent.click(screen.getByRole('tab', { name: 'Log esterni' }));
    expect(await screen.findByText('127.0.0.1:2239')).toBeInTheDocument();
  });

  it('shows connections: UDP, audio and data files', async () => {
    open();
    fireEvent.click(screen.getByRole('tab', { name: 'Collegamenti' }));
    expect(await screen.findByText('127.0.0.1:2334')).toBeInTheDocument();
    expect(screen.getByText('USB Audio')).toBeInTheDocument();
    expect(screen.getByText(/346 paesi/)).toBeInTheDocument();
    expect(screen.getByText(/2056 QSO/)).toBeInTheDocument();
  });

  it('offers one color picker for every kind of message, like JTDX', () => {
    open();
    for (const key of COLOR_KEYS) {
      expect(screen.getByLabelText(COLOR_LABELS[key])).toHaveValue(defaultColors()[key]);
    }
    expect(screen.getByLabelText(COLOR_LABELS.new_px)).toBeInTheDocument();
  });

  it('applies and remembers a chosen color right away', () => {
    open();
    fireEvent.change(screen.getByLabelText('Nuovo DXCC'), { target: { value: '#123456' } });
    expect(readColors(localStorage).new_dxcc).toBe('#123456');
    expect(document.documentElement.style.getPropertyValue('--tag-new-dxcc')).toBe('#123456');
  });

  it('restores the default colors', () => {
    open();
    fireEvent.change(screen.getByLabelText('Nuovo DXCC'), { target: { value: '#123456' } });
    fireEvent.click(screen.getByRole('button', { name: /ripristina i colori/i }));
    expect(screen.getByLabelText('Nuovo DXCC')).toHaveValue(defaultColors().new_dxcc);
    expect(readColors(localStorage).new_dxcc).toBe(defaultColors().new_dxcc);
  });

  it('restores the default window layout', () => {
    saveLayout(localStorage, resizePanel(defaultLayout(), 'cq', 500));
    open();
    fireEvent.click(screen.getByRole('tab', { name: 'Finestre' }));
    fireEvent.click(screen.getByRole('button', { name: /ripristina la disposizione/i }));
    expect(readLayout(localStorage)).toEqual(defaultLayout());
  });
});
