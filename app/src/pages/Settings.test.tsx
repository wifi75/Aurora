import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { kindKey, COLOR_KEYS, defaultColors, readColors } from '../lib/colors';
import { translate } from '../i18n';
import { useBandFilter } from '../lib/bandFilter';
import { defaultFilters, useFilters } from '../lib/filters';
import { defaultLayout, readLayout, resizePanel, saveLayout } from '../lib/layout';
import { defaultMacros, useMacros } from '../lib/macros';
import { defaultNotify, useNotify } from '../lib/notify';
import { defaultPrefs, usePrefs } from '../lib/prefs';

vi.mock('../api/client', () => ({
  getStationInfo: vi.fn().mockResolvedValue({
    tx_allowed: false,
    rig_control: true,
    export_udp: ['127.0.0.1:2239'],
    export_file: null,
    udp_listen: '127.0.0.1:2334',
    udp_forward: ['127.0.0.1:2333'],
    audio_device: 'USB Audio',
    cty_loaded: 346,
    worked_qsos: 2056,
    limits: { max_consecutive_cycles: 6, max_per_day: 40, dead_man_secs: 120, run_unattended: false },
  }),
  getClock: vi.fn().mockResolvedValue({ config: { enabled: true, servers: ['it.pool.ntp.org'], interval_secs: 900, warn_ms: 200, auto_correct: false }, last: null }),
  getEngineConfig: vi.fn().mockResolvedValue({ UDPServer: '127.0.0.1', UDPServerPort: '2334', AcceptUDPRequests: 'true', AcceptUDPReplyMessages: '1', NDepth: '3', SoundInName: 'Microfono di prova' }),
  getFrequencies: vi.fn().mockResolvedValue([{ band: '20m', mode: 'FT8', hz: 14_074_000 }]),
  putFrequencies: vi.fn().mockResolvedValue(undefined),
  getSchedule: vi.fn().mockResolvedValue({ enabled: false, entries: [] }),
  putSchedule: vi.fn().mockResolvedValue(undefined),
  getAutomation: vi.fn().mockResolvedValue({
    enabled: false, mode: 'dry_run', live_allowed: false,
    rules: { reply_to: ['new_dxcc'], min_snr: -18, excluded_calls: [], max_attempts_per_call: 2 },
    limits: { max_consecutive_cycles: 6, max_per_day: 40, dead_man_secs: 120, run_unattended: false },
    replies_today: 0, consecutive: 0, log: [],
  }),
  putAutomation: vi.fn().mockResolvedValue({}),
  getAccess: vi.fn().mockResolvedValue({ open: false }),
  putAccess: vi.fn().mockResolvedValue(undefined),
  getPairingSettings: vi.fn().mockResolvedValue({ code_ttl_secs: 600, min_secs: 60, max_secs: 3600 }),
  putPairingSettings: vi.fn().mockResolvedValue(undefined),
  getDevices: vi.fn().mockResolvedValue([{ id: 3, name: 'Telefono', created_at: 1 }]),
  revokeDevice: vi.fn().mockResolvedValue(undefined),
  startPairing: vi.fn().mockResolvedValue({ code: '12345678', expires_in: 600, qr_svg: '<svg/>' }),
  ApiError: class extends Error {},
}));
import * as api from '../api/client';
import { Settings } from './Settings';

const it_ = (key: Parameters<typeof translate>[1]) => translate('it', key);

beforeEach(() => {
  localStorage.clear();
  document.documentElement.removeAttribute('style');
  usePrefs.getState().reset();
  useNotify.getState().reset();
  useFilters.getState().reset();
  useMacros.getState().reset();
  useBandFilter.getState().reset();
  vi.mocked(api.putFrequencies).mockClear();
  vi.mocked(api.putSchedule).mockClear();
  vi.mocked(api.putAccess).mockClear();
  vi.mocked(api.putPairingSettings).mockClear();
});

function open() {
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <Settings />
    </QueryClientProvider>,
  );
}

const goTo = (name: string) => fireEvent.click(screen.getByRole('tab', { name }));

describe('Settings tabs', () => {
  it('opens straight on the tab asked by the link', () => {
    render(
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <Settings initialTab="clock" />
      </QueryClientProvider>,
    );
    expect(screen.getByRole('tab', { name: 'Orologio' })).toHaveAttribute('aria-selected', 'true');
  });

  it('has a separate tab for each configuration function, in the usual order, and shows one at a time', () => {
    open();
    const names = screen.getAllByRole('tab').map((t) => t.textContent);
    expect(names).toEqual([
      'Generali', 'Radio', 'Audio', 'Sequenza', 'Macro TX', 'Segnalazioni', 'Frequenze', 'Notifiche', 'Filtri',
      'Programmazione', 'Avanzate', 'DX Cluster', 'Desiderati', 'Servizi online', 'Aggiornamenti', 'Orologio', 'Accesso', 'Finestre', 'Bande',
    ]);
    expect(screen.getByRole('tab', { name: 'Generali' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('tabpanel')).toHaveAccessibleName('Generali');
    goTo('Finestre');
    expect(screen.getByRole('tabpanel')).toHaveAccessibleName('Finestre');
  });

  it('gives every tab a tooltip', () => {
    open();
    for (const tab of screen.getAllByRole('tab')) expect(tab.getAttribute('title'), tab.textContent ?? '').toBeTruthy();
  });
});

describe('General', () => {
  it('changes how messages are shown, at once', () => {
    open();
    fireEvent.click(screen.getByLabelText(it_('gen.miles')));
    expect(usePrefs.getState().prefs.distanceUnit).toBe('mi');
    fireEvent.click(screen.getByLabelText(it_('gen.showFlag')));
    expect(usePrefs.getState().prefs.showFlag).toBe(false);
    fireEvent.click(screen.getByLabelText(it_('gen.showDt')));
    expect(usePrefs.getState().prefs.showDt).toBe(false);
    fireEvent.click(screen.getByRole('button', { name: it_('set.reset') }));
    expect(usePrefs.getState().prefs).toEqual(defaultPrefs());
  });
});

describe('Radio, Audio, Reports and Advanced', () => {
  it('says whether the radio can be controlled from the page', async () => {
    open();
    goTo('Radio');
    expect(await screen.findByText('Attivo')).toBeInTheDocument();
  });

  it('shows the audio input', async () => {
    open();
    goTo('Audio');
    expect(await screen.findByText('USB Audio')).toBeInTheDocument();
  });

  it('shows the real the engine values: UDP server and port, audio device, decoding depth', async () => {
    open();
    goTo('Segnalazioni');
    expect(await screen.findByText('Server UDP primario: porta')).toBeInTheDocument();
    expect(screen.getByDisplayValue('2334')).toBeInTheDocument();
    expect(screen.getByText('solo CQ')).toBeInTheDocument();
    goTo('Audio');
    expect(await screen.findByDisplayValue('Microfono di prova')).toBeInTheDocument();
    goTo('Avanzate');
    expect(await screen.findByDisplayValue('profonda')).toBeInTheDocument();
  });

  it('shows the export targets, UDP, countries and QSOs', async () => {
    open();
    goTo('Segnalazioni');
    expect(await screen.findByText('127.0.0.1:2239')).toBeInTheDocument();
    expect(screen.getByText('127.0.0.1:2334')).toBeInTheDocument();
    expect(screen.getByText('346 paesi')).toBeInTheDocument();
    expect(screen.getByText('2056 QSO')).toBeInTheDocument();
  });

  it('explains that the decoder parameters live in the engine and links the guide', () => {
    open();
    goTo('Avanzate');
    expect(screen.getAllByText(/motore/).length).toBeGreaterThan(0);
    expect(screen.getByRole('link', { name: 'Leggi la guida' })).toHaveAttribute('href', '#/guida');
  });
});

describe('Sequence', () => {
  it('shows the automation rules', async () => {
    open();
    goTo('Sequenza');
    expect(await screen.findByLabelText(it_('auto.minSnr'))).toHaveValue(-18);
  });
});

describe('Macros', () => {
  it('adds and removes a macro and restores the five of the engine', () => {
    open();
    goTo('Macro TX');
    expect(screen.getByText('@ TNX 73')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText(it_('macro.new')), { target: { value: '73 GL' } });
    fireEvent.click(screen.getByRole('button', { name: it_('set.add') }));
    expect(useMacros.getState().macros).toContain('73 GL');
    fireEvent.click(screen.getByRole('button', { name: `${it_('set.remove')} 73 GL` }));
    expect(useMacros.getState().macros).not.toContain('73 GL');
    fireEvent.click(screen.getByRole('button', { name: it_('set.reset') }));
    expect(useMacros.getState().macros).toEqual(defaultMacros());
  });
});

describe('Frequencies', () => {
  it('lists the table and saves a change', async () => {
    open();
    goTo('Frequenze');
    const mhz = await screen.findByLabelText(it_('freq.mhz'));
    expect(mhz).toHaveValue(14.074);
    fireEvent.change(mhz, { target: { value: '14.075' } });
    fireEvent.click(screen.getByRole('button', { name: it_('set.save') }));
    await waitFor(() => expect(api.putFrequencies).toHaveBeenCalledWith([{ band: '20m', mode: 'FT8', hz: 14_075_000 }]));
  });
});

describe('Notifications', () => {
  it('has a color, a highlight and a sound switch for every kind, like the old one', () => {
    open();
    goTo('Notifiche');
    for (const key of COLOR_KEYS) {
      const name = it_(kindKey(key));
      expect(screen.getByLabelText(name)).toHaveValue(defaultColors()[key]);
      expect(screen.getByLabelText(`${it_('not.highlight')}: ${name}`)).toBeChecked();
    }
    expect(screen.getByLabelText(it_('kind.new_px'))).toBeInTheDocument();
  });

  it('applies and remembers a color right away', () => {
    open();
    goTo('Notifiche');
    fireEvent.change(screen.getByLabelText(it_('kind.new_dxcc')), { target: { value: '#123456' } });
    expect(readColors(localStorage).new_dxcc).toBe('#123456');
    expect(document.documentElement.style.getPropertyValue('--tag-new-dxcc')).toBe('#123456');
  });

  it('switches highlight and sound per kind, and restores everything', () => {
    open();
    goTo('Notifiche');
    fireEvent.click(screen.getByLabelText(`${it_('not.highlight')}: ${it_('kind.new_call')}`));
    expect(useNotify.getState().notify.highlight.new_call).toBe(false);
    fireEvent.click(screen.getByLabelText(`${it_('not.sound')}: ${it_('kind.cq')}`));
    expect(useNotify.getState().notify.sound.cq).toBe(true);
    fireEvent.click(screen.getByRole('button', { name: it_('not.restoreColors') }));
    expect(useNotify.getState().notify).toEqual(defaultNotify());
  });
});

describe('Filters', () => {
  it('hides continents, countries and calls and chooses what to show only', () => {
    open();
    goTo('Filtri');
    fireEvent.click(screen.getByLabelText('Asia'));
    expect(useFilters.getState().filters.hideContinents).toEqual(['AS']);
    fireEvent.change(screen.getByLabelText(it_('fil.countries')), { target: { value: 'Japan, Italy' } });
    expect(useFilters.getState().filters.hideCountries).toEqual(['Japan', 'Italy']);
    fireEvent.change(screen.getByLabelText(it_('fil.calls')), { target: { value: 'JA*' } });
    expect(useFilters.getState().filters.hideCalls).toEqual(['JA*']);
    fireEvent.change(screen.getByLabelText(it_('fil.only')), { target: { value: 'cq' } });
    expect(useFilters.getState().filters.only).toBe('cq');
    fireEvent.click(screen.getByLabelText(it_('fil.nonStandard')));
    expect(useFilters.getState().filters.hideNonStandard).toBe(true);
    fireEvent.click(screen.getByRole('button', { name: it_('set.reset') }));
    expect(useFilters.getState().filters).toEqual(defaultFilters());
  });
});

describe('Schedule', () => {
  it('saves a row with the schedule switched on', async () => {
    open();
    goTo('Programmazione');
    await screen.findByLabelText(`${it_('sch.hour')} 1`);
    fireEvent.click(screen.getByLabelText(it_('sch.use')));
    fireEvent.change(screen.getByLabelText(`${it_('sch.hour')} 1`), { target: { value: '6' } });
    fireEvent.change(screen.getByLabelText(`${it_('sch.minute')} 1`), { target: { value: '30' } });
    fireEvent.change(screen.getByLabelText(`${it_('sch.band')} 1`), { target: { value: '40m' } });
    fireEvent.click(screen.getByRole('button', { name: it_('set.save') }));
    await waitFor(() => expect(api.putSchedule).toHaveBeenCalledWith({ enabled: true, entries: [{ hour: 6, minute: 30, band: '40m' }] }));
  });
});

describe('Access', () => {
  it('switches the code-free access on, only for tests', async () => {
    open();
    goTo('Accesso');
    const box = await screen.findByLabelText(it_('acc.open'));
    fireEvent.click(box);
    await waitFor(() => expect(api.putAccess).toHaveBeenCalledWith(true));
    expect(screen.getByText(/Attenzione/)).toBeInTheDocument();
  });

  it('lets the operator choose how many minutes a pairing code lasts', async () => {
    open();
    goTo('Accesso');
    const input = await screen.findByLabelText(it_('acc.ttl'));
    await waitFor(() => expect(input).toHaveValue(10));
    fireEvent.change(input, { target: { value: '15' } });
    fireEvent.click(screen.getByRole('button', { name: it_('set.save') }));
    await waitFor(() => expect(api.putPairingSettings).toHaveBeenCalledWith(900));
  });

  it('does not save a lifetime outside the limits', async () => {
    open();
    goTo('Accesso');
    const input = await screen.findByLabelText(it_('acc.ttl'));
    fireEvent.change(input, { target: { value: '90' } });
    expect(screen.getByRole('button', { name: it_('set.save') })).toBeDisabled();
    fireEvent.change(input, { target: { value: '0' } });
    expect(screen.getByRole('button', { name: it_('set.save') })).toBeDisabled();
  });

  it('shows a new pairing code with its link and lists the devices', async () => {
    open();
    goTo('Accesso');
    fireEvent.click(await screen.findByRole('button', { name: it_('acc.showCode') }));
    expect(await screen.findByText('12345678')).toBeInTheDocument();
    expect(screen.getByText(/#\/pair\/12345678/)).toBeInTheDocument();
    expect(await screen.findByText('Telefono')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: `${it_('acc.revoke')} Telefono` }));
    await waitFor(() => expect(api.revokeDevice).toHaveBeenCalledWith(3));
  });
});

describe('Windows and Bands', () => {
  it('restores the default window layout', () => {
    saveLayout(localStorage, resizePanel(defaultLayout(), 'cq', 500));
    open();
    goTo('Finestre');
    fireEvent.click(screen.getByRole('button', { name: it_('win.reset') }));
    expect(readLayout(localStorage)).toEqual(defaultLayout());
  });

  it('lets the user choose which bands the station page shows, with immediate effect', () => {
    open();
    goTo('Bande');
    const box = screen.getByLabelText('160m');
    expect(box).toBeChecked();
    fireEvent.click(box);
    expect(box).not.toBeChecked();
    expect(useBandFilter.getState().bands).not.toContain('160m');
    fireEvent.click(screen.getByRole('button', { name: it_('bands.all') }));
    expect(useBandFilter.getState().bands).toContain('160m');
  });
});
