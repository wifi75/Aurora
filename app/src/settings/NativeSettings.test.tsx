import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({
  getStationInfo: vi.fn(),
  getEngineSettings: vi.fn(),
  putEngineSettings: vi.fn(),
  getEngineConfig: vi.fn(),
  applyEngineConfig: vi.fn(),
  ApiError: class extends Error {
    constructor(
      readonly status: number,
      readonly code: string,
    ) {
      super(code);
    }
  },
}));
import * as api from '../api/client';
import type { EngineSettings } from '../api/schemas';
import { translate } from '../i18n';
import { EngineValues } from './EngineValues';
import { ReportsTab } from './InfoTabs';

const it_ = (key: Parameters<typeof translate>[1], vars?: Record<string, string | number>) => translate('it', key, vars);

const info = { engine: 'native', tx_allowed: true, udp_listen: '127.0.0.1:2334', udp_forward: [], export_udp: [], export_file: null, audio_device: null, cty_loaded: 0, worked_qsos: 0, limits: { max_consecutive_cycles: 6, max_per_day: 40, dead_man_secs: 120, run_unattended: false } };

function doc(): EngineSettings {
  const live = { state: 'live', apply: 'now' } as const;
  const off = (reason: string) => ({ state: 'unavailable', reason }) as const;
  return {
    version: 1,
    file: 'settings-motore.json',
    values: { MyCall: 'IU3ABC', MyGrid: 'IO91wm', UDP2Server: '127.0.0.1', UDP2ServerPort: '2237', EnableUDP2adifBroadcast: 'true', AllowSpotsDXSummit: 'true' },
    status: { dxsummit: { ok: true, at_ms: 1_792_860_405_000, message: 'spot inviato: DL1ABC su 14075.5 kHz', sent: 1, failed: 0 } },
    items: {
      MyCall: { state: 'live', apply: 'restart' },
      MyGrid: { state: 'live', apply: 'restart' },
      UDP2Server: live,
      UDP2ServerPort: live,
      EnableUDP2adifBroadcast: live,
      TCPServer: off('tcp'),
      TCPServerPort: off('tcp'),
      EnableTCPConnection: off('tcp'),
      PSKReporter: off('psk'),
      AllowSpotsDXSummit: live,
      EQSLSend: off('eqsl'),
      EQSLUser: off('eqsl'),
      EQSLPasswd: off('eqsl'),
      EQSLNick: off('eqsl'),
      PromptToLog: off('log_prompt'),
      AutoQSOLogging: off('log_auto'),
      toRTTY: off('adif_options'),
      dBtoComments: off('adif_options'),
      distanceToComments: off('adif_options'),
      ClearCallGrid: off('clear_dx'),
      ClearCallGridExit: off('clear_dx'),
      WriteDecodedALLTXT: off('all_txt_pending'),
      WriteDecodedDebugALLTXT: off('all_txt_pending'),
      UDPServer: off('emitter_pending'),
      UDPServerPort: off('emitter_pending'),
      AcceptUDPRequests: off('emitter_pending'),
      AcceptUDPReplyMessages: off('emitter_pending'),
      udpWindowToFront: off('udp_window'),
      udpWindowRestore: off('udp_window'),
      EnableUDP1adifSending: off('emitter_pending'),
      preventFalseUDPspots: off('false_spots'),
      ApplyFiltersToUDPmessages: off('udp_filters'),
      Rig: off('radio_cat'),
    },
  };
}

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  vi.mocked(api.getStationInfo).mockReset().mockResolvedValue(info);
  vi.mocked(api.getEngineSettings).mockReset().mockResolvedValue(doc());
  vi.mocked(api.putEngineSettings).mockReset();
  vi.mocked(api.getEngineConfig).mockReset().mockResolvedValue({});
});

describe('Reports tab with the built-in engine', () => {
  it('has the same five groups as JTDX, with the voices of its screen', async () => {
    render(<ReportsTab />);
    for (const title of ['ens.g.log', 'ens.g.adif', 'ens.g.alltxt', 'ens.g.net', 'ens.g.udp'] as const) expect(await screen.findByText(it_(title))).toBeInTheDocument();
    for (const key of ['PromptToLog', 'toRTTY', 'dBtoComments', 'ClearCallGridExit', 'TCPServer', 'UDP2Server', 'WriteDecodedDebugALLTXT', 'EQSLNick', 'PSKReporter', 'AllowSpotsDXSummit', 'UDPServer', 'preventFalseUDPspots', 'ApplyFiltersToUDPmessages'] as const) {
      expect(screen.getByLabelText(it_(`ens.k.${key}`))).toBeInTheDocument();
    }
    // niente del vecchio testo «file del motore» e «riavvia il motore»
    expect(screen.queryByText(/riavvia il motore/i)).toBeNull();
    expect(screen.queryByText(/AURORA_EXPORT_UDP/)).toBeNull();
    expect(api.getEngineConfig).not.toHaveBeenCalled();
  });

  it('has the DXSummit spotting as a working voice, on, with the last send and the way to spot', async () => {
    render(<ReportsTab />);
    const box = await screen.findByLabelText(it_('ens.k.AllowSpotsDXSummit'));
    expect(box).toBeEnabled();
    expect(box).toBeChecked();
    expect(screen.getByTestId('status-dxsummit')).toHaveTextContent(/spot inviato: DL1ABC su 14075.5 kHz/);
    expect(screen.getByText(it_('ens.dxsummit.help'))).toBeInTheDocument();
  });

  it('shows what Aurora cannot do yet as switched off, with the reason, and never as a working control', async () => {
    render(<ReportsTab />);
    const psk = await screen.findByLabelText(it_('ens.k.PSKReporter'));
    expect(psk).toBeDisabled();
    expect(psk).not.toBeChecked();
    expect(screen.getAllByText(it_('ens.r.psk')).length).toBeGreaterThan(0);
    expect(screen.getByLabelText(it_('ens.k.TCPServerPort'))).toBeDisabled();
    expect(screen.getByLabelText(it_('ens.k.EQSLPasswd'))).toBeDisabled();
    expect(screen.getByLabelText(it_('ens.k.UDP2Server'))).toBeEnabled();
    expect(screen.getByLabelText(it_('ens.k.UDP2Server'))).toHaveValue('127.0.0.1');
    expect(screen.getByLabelText(it_('ens.k.EnableUDP2adifBroadcast'))).toBeChecked();
  });

  it('saves a change at once, with no restart, and shows the new values', async () => {
    const next = doc();
    next.values.UDP2ServerPort = '2240';
    vi.mocked(api.putEngineSettings).mockResolvedValue(next);
    render(<ReportsTab />);
    fireEvent.change(await screen.findByLabelText(it_('ens.k.UDP2ServerPort')), { target: { value: '2240' } });
    expect(screen.getByText(it_('ens.pending', { n: 1 }))).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: it_('ens.apply') }));
    await waitFor(() => expect(api.putEngineSettings).toHaveBeenCalledWith({ UDP2ServerPort: '2240' }));
    expect(await screen.findByText(it_('ens.applied'))).toBeInTheDocument();
    expect(screen.getByLabelText(it_('ens.k.UDP2ServerPort'))).toHaveValue(2240);
  });

  it('says so when the core refuses a value', async () => {
    vi.mocked(api.putEngineSettings).mockRejectedValue(new (api.ApiError as unknown as new (s: number, c: string) => Error)(400, 'bad_setting'));
    render(<ReportsTab />);
    fireEvent.change(await screen.findByLabelText(it_('ens.k.UDP2ServerPort')), { target: { value: '99999' } });
    fireEvent.click(screen.getByRole('button', { name: it_('ens.apply') }));
    expect(await screen.findByText(it_('ens.err.bad'))).toBeInTheDocument();
  });
});

describe('Other tabs with the built-in engine', () => {
  it('General shows only what Aurora really uses and lists the rest with the reason', async () => {
    render(<EngineValues group="general" />);
    expect(await screen.findByLabelText(it_('jx.MyCall'))).toHaveValue('IU3ABC');
    expect(screen.getAllByText(it_('ens.restart'), { exact: false }).length).toBe(2);
    expect(screen.queryByLabelText(it_('jx.TxWatchdogTimer'))).toBeNull();
  });

  it('Radio has nothing of another program’s file: only the explanation', async () => {
    render(<EngineValues group="radio" />);
    expect(await screen.findByText(it_('ens.hidden', { n: 8 }))).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: it_('ens.apply') })).toBeNull();
  });

  it('with JTDX as the engine the old behaviour stays', async () => {
    vi.mocked(api.getStationInfo).mockResolvedValue({ ...info, engine: 'jtdx' });
    vi.mocked(api.getEngineConfig).mockResolvedValue({ MyCall: 'IU3ABC' });
    render(<EngineValues group="general" />);
    expect(await screen.findByLabelText(it_('jx.MyCall'))).toHaveValue('IU3ABC');
    expect(api.getEngineSettings).not.toHaveBeenCalled();
  });
});
