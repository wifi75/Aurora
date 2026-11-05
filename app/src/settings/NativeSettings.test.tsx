import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({
  getStationInfo: vi.fn(),
  getEngineSettings: vi.fn(),
  putEngineSettings: vi.fn(),
  postUploadTest: vi.fn(),
  getOnline: vi.fn(() => Promise.resolve({ psk: false, solar: false })),
  putOnline: vi.fn(),
  ApiError: class extends Error {
    constructor(
      readonly status: number,
      readonly code: string,
      readonly retryAfterSecs?: number,
      readonly detail?: string,
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
import { LogServicesTab } from './LogServicesTab';
import { ledOf } from './NativeSettings';
import { OnlineTab } from './OnlineTab';

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
});

describe('Connected programs tab with the built-in engine', () => {
  it('has only the links to other programs, in order, and no box of unavailable items', async () => {
    render(<ReportsTab />);
    expect(await screen.findByText(it_('ens.g.adif'))).toBeInTheDocument();
    expect(screen.getByLabelText(it_('ens.k.UDP2Server'))).toHaveValue('127.0.0.1');
    expect(screen.getByLabelText(it_('ens.k.EnableUDP2adifBroadcast'))).toBeChecked();
    // le voci che Aurora non usa non compaiono, neanche spente
    for (const key of ['TCPServer', 'UDPServer', 'PSKReporter', 'EQSLUser', 'PromptToLog', 'WriteDecodedALLTXT'] as const) expect(screen.queryByLabelText(it_(`ens.k.${key}`))).toBeNull();
    expect(screen.queryByText(/Non disponibili/)).toBeNull();
    // i servizi di log e gli spot hanno le loro schede
    expect(screen.queryByText(it_('ens.g.qrz'))).toBeNull();
    expect(screen.queryByLabelText(it_('ens.k.AllowSpotsDXSummit'))).toBeNull();
    expect(screen.queryByText(/riavvia il motore/i)).toBeNull();
  });

  it('has the DXSummit spotting in Online services, on, with the last send', async () => {
    render(<OnlineTab />);
    const box = await screen.findByLabelText(it_('ens.k.AllowSpotsDXSummit'));
    expect(box).toBeEnabled();
    expect(box).toBeChecked();
    // in breve l'esito; il dettaglio (cosa e' stato inviato) sta nel suggerimento
    expect(screen.getByTestId('status-dxsummit').getAttribute('title')).toMatch(/Spot inviato: DL1ABC su 14075.5 kHz/);
    expect(screen.queryByLabelText(it_('ens.k.PSKReporter'))).toBeNull();
  });

  it('shows the real reason of a refusal next to the field, with the name of the field', async () => {
    vi.mocked(api.putEngineSettings).mockRejectedValue(new (api.ApiError as unknown as new (s: number, c: string, r?: number, d?: string) => Error)(400, 'bad_setting', undefined, "UDP2Server: serve un indirizzo IP (o localhost)"));
    render(<ReportsTab />);
    fireEvent.change(await screen.findByLabelText(it_('ens.k.UDP2Server')), { target: { value: 'x' } });
    fireEvent.click(screen.getByRole('button', { name: it_('ens.apply') }));
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent(`${it_('ens.k.UDP2Server')}: serve un indirizzo IP (o localhost)`);
    expect(screen.getByLabelText(it_('ens.k.UDP2Server')).closest('label')).toContainElement(alert);
    expect(screen.queryByText(it_('ens.err.bad'))).toBeNull();
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

  it('Radio has nothing of another program’s file and no list of unavailable items', async () => {
    render(<EngineValues group="radio" />);
    expect(await screen.findByText(it_('ens.title'))).toBeInTheDocument();
    expect(screen.queryByText(/Non disponibili/)).toBeNull();
    expect(screen.queryByRole('button', { name: it_('ens.apply') })).toBeNull();
  });
});

describe('Direct QSO upload with the built-in engine', () => {
  function withQrz(): EngineSettings {
    const d = doc();
    return {
      ...d,
      values: { ...d.values, QRZLogSend: 'true' },
      secrets: { QRZLogKey: true },
      items: { ...d.items, QRZLogSend: { state: 'live', apply: 'now' }, QRZLogKey: { state: 'live', apply: 'now' }, WavelogSend: { state: 'unavailable', reason: 'upload_secrets' } },
      status: { ...d.status, qrz: { ok: false, at_ms: 1_792_860_405_000, message: 'QRZ ha risposto 503: nuovo tentativo alle 17:47 UTC', sent: 0, failed: 1, pending: 2 } },
    };
  }

  it('shows the switch, the saved key, the queue and the test button with its answer', async () => {
    vi.mocked(api.getEngineSettings).mockResolvedValue(withQrz());
    vi.mocked(api.postUploadTest).mockReset().mockResolvedValue('chiave valida, logbook K1ABC');
    render(<LogServicesTab />);
    expect(await screen.findByText(it_('ens.g.qrz'))).toBeInTheDocument();
    expect(screen.getByLabelText(it_('ens.k.QRZLogSend'))).toBeChecked();
    expect(screen.getByTestId('ens-QRZLogKey-saved')).toHaveTextContent(it_('ens.pw.savedShort'));
    expect(screen.getByTestId('status-qrz')).toHaveTextContent(it_('ens.st.pending', { n: 2 }));
    // la spiegazione lunga sta nel «?», non sotto i campi
    expect(screen.queryByText(it_('ens.up.qrz.help'))).toBeNull();
    fireEvent.click(screen.getByTestId('test-qrz'));
    await waitFor(() => expect(api.postUploadTest).toHaveBeenCalledWith('qrz'));
    expect((await screen.findByTestId('test-result-qrz')).getAttribute('title')).toBe('chiave valida, logbook K1ABC');
    // senza la protezione dei segreti Wavelog non e' un controllo vivo
    expect(screen.queryByTestId('test-wavelog')).toBeNull();
  });
  it('a new key is saved only on Enter, then shown as saved and tested by itself', async () => {
    const before = withQrz();
    const after: EngineSettings = { ...before, secrets: { QRZLogKey: true } };
    vi.mocked(api.getEngineSettings).mockResolvedValue({ ...before, secrets: {} });
    vi.mocked(api.putEngineSettings).mockImplementation(async () => {
      vi.mocked(api.getEngineSettings).mockResolvedValue(after);
      return after;
    });
    vi.mocked(api.postUploadTest).mockReset().mockResolvedValue('chiave valida');
    render(<LogServicesTab />);
    const field = await screen.findByLabelText(it_('ens.k.QRZLogKey'));
    fireEvent.change(field, { target: { value: 'ABCD-1234' } });
    // mentre si scrive non parte niente
    await new Promise((r) => setTimeout(r, 900));
    expect(api.putEngineSettings).not.toHaveBeenCalled();
    fireEvent.keyDown(field, { key: 'Enter' });
    await waitFor(() => expect(api.putEngineSettings).toHaveBeenCalledWith({ QRZLogKey: 'ABCD-1234' }));
    expect(await screen.findByTestId('ens-QRZLogKey-saved')).toBeInTheDocument();
    await waitFor(() => expect(api.postUploadTest).toHaveBeenCalledWith('qrz'));
    expect((await screen.findByTestId('test-result-qrz')).getAttribute('title')).toMatch(/chiave valida/);
  });

  it('the title LED goes red with the reason, green after a passing test', async () => {
    vi.mocked(api.getEngineSettings).mockResolvedValue(withQrz());
    vi.mocked(api.postUploadTest).mockReset().mockRejectedValue(new (api.ApiError as unknown as new (s: number, c: string, r?: number, d?: string) => Error)(502, 'upload_test_failed', undefined, 'QRZ ha risposto 503'));
    render(<LogServicesTab />);
    const qrzLed = () => screen.getAllByTestId('ens-led').find((l) => l.getAttribute('title')?.includes('503'));
    await waitFor(() => expect(qrzLed()).toHaveAttribute('data-led', 'err'));
    vi.mocked(api.postUploadTest).mockReset().mockResolvedValue('chiave valida');
    fireEvent.click(screen.getByTestId('test-qrz'));
    await waitFor(() => expect(screen.getAllByTestId('ens-led').some((l) => l.getAttribute('data-led') === 'ok')).toBe(true));
    // spento = grigio, acceso e mai provato = giallo
    expect(ledOf(false, null, undefined).led).toBe('off');
    expect(ledOf(true, null, { ok: null, message: '' }).led).toBe('untested');
  });

  it('«Remove» deletes the saved key', async () => {
    vi.mocked(api.getEngineSettings).mockResolvedValue(withQrz());
    vi.mocked(api.putEngineSettings).mockResolvedValue({ ...withQrz(), secrets: {} });
    render(<LogServicesTab />);
    fireEvent.click(await screen.findByText(it_('ens.pw.remove')));
    await waitFor(() => expect(api.putEngineSettings).toHaveBeenCalledWith({ QRZLogKey: '' }));
  });
});
