import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor, within } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({
  getStationInfo: vi.fn(),
  getEngineSettings: vi.fn(),
  putEngineSettings: vi.fn(),
  getEngineConfig: vi.fn(),
  applyEngineConfig: vi.fn(),
  getAutomation: vi.fn(),
  getControls: vi.fn(),
  postControl: vi.fn(),
  putAutomation: vi.fn(),
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
import { useNotify } from '../lib/notify';
import { useNotifyView } from '../lib/notifyView';
import { usePrefs } from '../lib/prefs';
import { NativeGeneral, NativeNotifications, NativeSequence } from './JtdxTabs';

const it_ = (key: Parameters<typeof translate>[1], vars?: Record<string, string | number>) => translate('it', key, vars);

const info = { engine: 'native', tx_allowed: true, udp_listen: '127.0.0.1:2334', udp_forward: [], export_udp: [], export_file: null, audio_device: null, cty_loaded: 0, worked_qsos: 0, limits: { max_consecutive_cycles: 6, max_per_day: 40, dead_man_secs: 120, run_unattended: false } };

const LIVE_NOW = [
  'SeqAnswerCQCount', 'SeqAnswerCQCounterValue', 'SeqAnswerInCallCount', 'SeqAnswerInCallCounterValue', 'SeqSentRReportCount', 'SeqSentRReportCounterValue', 'SeqSentRR7373Count',
  'SeqSentRR7373CounterValue', 'SeqHaltTxReplyOther', 'StrictDirectionalCQ', 'CallPriorityCQ', 'MaxDistance', 'TxWatchdogTimer', 'TxQSYAllowed', 'callsignLogFiltering', 'gridLogFiltering', 'timeFromLogFiltering', 'newPotential', '73RR73Marker',
  ...['newCQZ', 'newITUZ', 'newDXCC', 'newGrid', 'newPx', 'newCall'].flatMap((k) => [k, `${k}Band`, `${k}BandMode`]),
];

function doc(values: Record<string, string> = {}): EngineSettings {
  const items: EngineSettings['items'] = {};
  for (const k of LIVE_NOW) items[k] = { state: 'live', apply: 'now' };
  items.MyCall = { state: 'live', apply: 'restart' };
  items.MyGrid = { state: 'live', apply: 'restart' };
  items.MonitorOFF = { state: 'live', apply: 'restart' };
  const base: Record<string, string> = {
    MyCall: 'IU3ABC', MyGrid: 'IO91wm', MonitorOFF: 'false',
    SeqAnswerCQCount: 'false', SeqAnswerCQCounterValue: '2', SeqAnswerInCallCount: 'false', SeqAnswerInCallCounterValue: '3',
    SeqSentRReportCount: 'true', SeqSentRReportCounterValue: '5', SeqSentRR7373Count: 'true', SeqSentRR7373CounterValue: '5',
    SeqHaltTxReplyOther: 'true', StrictDirectionalCQ: 'false', TxWatchdogTimer: '6', TxQSYAllowed: 'true',
    callsignLogFiltering: 'false', gridLogFiltering: 'false', timeFromLogFiltering: '', newPotential: 'false', '73RR73Marker': 'true',
  };
  for (const k of ['newCQZ', 'newITUZ', 'newDXCC', 'newGrid', 'newPx', 'newCall']) {
    const on = k === 'newDXCC' || k === 'newCall';
    base[k] = String(on);
    base[`${k}Band`] = String(on);
    base[`${k}BandMode`] = String(on);
  }
  return { version: 1, file: 'settings-motore.json', values: { ...base, ...values }, items };
}

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  vi.mocked(api.getStationInfo).mockReset().mockResolvedValue(info);
  vi.mocked(api.getEngineSettings).mockReset().mockResolvedValue(doc());
  vi.mocked(api.putEngineSettings).mockReset();
  vi.mocked(api.getControls).mockReset().mockResolvedValue({ autoseq: true, single_qso: false, skip_tx1: true, answer_b4: false });
  vi.mocked(api.postControl).mockReset().mockResolvedValue(undefined as never);
  useNotifyView.getState().reset();
  useNotify.getState().reset();
  usePrefs.getState().reset();
});

describe('Sequence tab (as in JTDX)', () => {
  it('has the four short scenarios, each with its counter, plus the two stop options', async () => {
    render(<NativeSequence />);
    for (const label of ['seq2.answerCq', 'seq2.answerInCall', 'seq2.sentRReport', 'seq2.sentRR7373'] as const) {
      expect(await screen.findByLabelText(it_(label))).toBeInTheDocument();
    }
    expect(screen.getByLabelText(it_('seq2.haltReplyOther'))).toBeChecked();
    expect(screen.getByLabelText(it_('seq2.strictCq'))).not.toBeChecked();
    // acceso a 5 come la configurazione di sempre; spento = numero fermo
    expect(screen.getByLabelText(`${it_('seq2.sentRReport')} (${it_('seq2.times')})`)).toHaveValue(5);
    expect(screen.getByLabelText(`${it_('seq2.sentRReport')} (${it_('seq2.times')})`)).toBeEnabled();
    expect(screen.getByLabelText(`${it_('seq2.answerCq')} (${it_('seq2.times')})`)).toBeDisabled();
  });

  it('sends the changed counters together with Apply and takes effect without a restart', async () => {
    vi.mocked(api.putEngineSettings).mockResolvedValue(doc({ SeqAnswerCQCount: 'true', SeqAnswerCQCounterValue: '3' }));
    render(<NativeSequence />);
    fireEvent.click(await screen.findByLabelText(it_('seq2.answerCq')));
    fireEvent.change(screen.getByLabelText(`${it_('seq2.answerCq')} (${it_('seq2.times')})`), { target: { value: '3' } });
    fireEvent.click(screen.getByLabelText(it_('seq2.strictCq')));
    fireEvent.click(screen.getByRole('button', { name: it_('ens.apply') }));
    await waitFor(() => expect(api.putEngineSettings).toHaveBeenCalledWith({ SeqAnswerCQCount: 'true', SeqAnswerCQCounterValue: '3', StrictDirectionalCQ: 'true' }));
    expect(await screen.findByText(it_('ens.applied'))).toBeInTheDocument();
  });
});

describe('Sequence tab switches', () => {
  it('shows and changes the very same state as the QSO panel (the controls), and sends the two priorities with Apply', async () => {
    vi.mocked(api.putEngineSettings).mockResolvedValue(doc({ CallPriorityCQ: 'true', MaxDistance: 'true' }));
    render(<NativeSequence />);
    const autoseq = await screen.findByLabelText(it_('seq2.autoseq'));
    await waitFor(() => expect(autoseq).toBeChecked());
    expect(screen.getByLabelText(it_('seq2.skipTx1'))).toBeChecked();
    expect(screen.getByLabelText(it_('seq2.single'))).not.toBeChecked();
    fireEvent.click(screen.getByLabelText(it_('seq2.single')));
    await waitFor(() => expect(api.postControl).toHaveBeenCalledWith('single_qso', true));
    fireEvent.click(screen.getByLabelText(it_('seq2.prioCq')));
    fireEvent.click(screen.getByLabelText(it_('seq2.maxDist')));
    fireEvent.click(screen.getByRole('button', { name: it_('ens.apply') }));
    await waitFor(() => expect(api.putEngineSettings).toHaveBeenCalledWith({ CallPriorityCQ: 'true', MaxDistance: 'true' }));
  });
});

describe('Notifications tab (as in JTDX)', () => {
  it('has the six «new» rows with per band, per mode and also a sound, dependent on the first box', async () => {
    render(<NativeNotifications />);
    const cqz = await screen.findByLabelText(it_('not2.cqz'));
    expect(cqz).not.toBeChecked();
    // «per banda» e «per modo» della riga spenta sono bloccati; quella accesa (paese) no
    const grid = document.querySelector('.jt-grid') as HTMLElement;
    const boxes = ['newCQZBand', 'newITUZBand', 'newDXCCBand'].map((k) => grid.querySelector(`[data-setting="${k}"]`) as HTMLInputElement);
    expect(boxes[0]).toBeDisabled();
    expect(boxes[2]).toBeEnabled();
    expect(boxes[2]).toBeChecked();
    fireEvent.click(cqz);
    expect(boxes[0]).toBeEnabled();
  });

  it('sends the highlights with Apply (never the display choices)', async () => {
    vi.mocked(api.putEngineSettings).mockResolvedValue(doc({ newCQZ: 'true', newCQZBand: 'true' }));
    render(<NativeNotifications />);
    fireEvent.click(await screen.findByLabelText(it_('not2.cqz')));
    const grid = document.querySelector('.jt-grid') as HTMLElement;
    fireEvent.click(within(grid).getAllByLabelText(it_('not2.perBand'))[0]!);
    fireEvent.click(screen.getByLabelText(it_('not2.strike')));
    fireEvent.click(screen.getByRole('button', { name: it_('ens.apply') }));
    await waitFor(() => expect(api.putEngineSettings).toHaveBeenCalledWith({ newCQZ: 'true', newCQZBand: 'true' }));
  });

  it('worked options: strike by default, underline excludes strike, they need at least one «new» highlight', async () => {
    render(<NativeNotifications />);
    const strike = await screen.findByLabelText(it_('not2.strike'));
    const underline = screen.getByLabelText(it_('not2.underline'));
    expect(strike).toBeChecked();
    expect(underline).toBeDisabled();
    fireEvent.click(strike);
    expect(useNotifyView.getState().view.workedStriked).toBe(false);
    expect(underline).toBeEnabled();
    fireEvent.click(underline);
    expect(useNotifyView.getState().view.workedUnderlined).toBe(true);
    expect(screen.getByLabelText(it_('not2.strike'))).toBeDisabled();
    fireEvent.click(screen.getByLabelText(it_('not2.hide')));
    expect(useNotifyView.getState().view.workedDontShow).toBe(true);
    fireEvent.click(screen.getByLabelText(it_('not2.invert')));
    expect(useNotifyView.getState().view.invert).toBe(true);
  });

  it('with no «new» highlight on the worked options are off (there is no «already worked» without them)', async () => {
    vi.mocked(api.getEngineSettings).mockResolvedValue(doc({ newDXCC: 'false', newCall: 'false' }));
    render(<NativeNotifications />);
    expect(await screen.findByLabelText(it_('not2.strike'))).toBeDisabled();
    expect(screen.getByLabelText(it_('not2.strike'))).not.toBeChecked();
    expect(screen.getByLabelText(it_('not2.hide'))).not.toBeChecked();
  });

  it('shows the worked preview struck, and underlined when chosen', async () => {
    render(<NativeNotifications />);
    await screen.findByLabelText(it_('not2.strike'));
    const sample = () => [...document.querySelectorAll('.jt-sample')].pop() as HTMLElement;
    expect(sample()).toHaveClass('jt-sample--strike');
    fireEvent.click(screen.getByLabelText(it_('not2.strike')));
    fireEvent.click(screen.getByLabelText(it_('not2.underline')));
    expect(sample()).toHaveClass('jt-sample--underline');
    expect(sample()).not.toHaveClass('jt-sample--strike');
  });

  it('every voice of the tab works: the only one switched off is the marker for hints, with its reason', async () => {
    render(<NativeNotifications />);
    expect(await screen.findByLabelText(it_('not2.markHint'))).toBeDisabled();
    expect(screen.getAllByText(it_('ens.r.no_hint')).length).toBeGreaterThan(0);
    expect(screen.getByLabelText(it_('not2.alertDx'))).toBeEnabled();
    expect(screen.getByLabelText(it_('not2.alertMy'))).toBeChecked();
    const blue = screen.getByLabelText(it_('not2.markBlue'));
    expect(blue).toBeEnabled();
    fireEvent.click(screen.getByLabelText(it_('not2.markRed')));
    expect(blue).toBeDisabled();
  });

  it('the alerts of each row (play alert sound, per band) are saved at once and «per band» needs the alert', async () => {
    render(<NativeNotifications />);
    await screen.findByLabelText(it_('not2.cqz'));
    const grid = document.querySelector('.jt-grid') as HTMLElement;
    const plays = within(grid).getAllByLabelText(it_('not2.playAlert'));
    expect(plays).toHaveLength(5);
    const dxccOb = grid.querySelector('[data-alert="alertDxccOb"]') as HTMLInputElement;
    expect(dxccOb).toBeDisabled();
    fireEvent.click(plays[2]!);
    expect(useNotifyView.getState().view.alertDxcc).toBe(true);
    expect(dxccOb).toBeEnabled();
    fireEvent.click(dxccOb);
    expect(useNotifyView.getState().view.alertDxccOb).toBe(true);
    fireEvent.click(plays[2]!);
    expect(useNotifyView.getState().view).toMatchObject({ alertDxcc: false, alertDxccOb: false });
    fireEvent.click(screen.getByLabelText(it_('not2.alertDx')));
    expect(useNotifyView.getState().view.alertDxCall).toBe(true);
  });

  it('chooses the sound, the volume and the silence while transmitting, and has a test button', async () => {
    render(<NativeNotifications />);
    const pick = await screen.findByLabelText(it_('not2.soundPick'));
    fireEvent.change(pick, { target: { value: 'horn' } });
    expect(JSON.parse(localStorage.getItem('aurora-alerts') ?? '{}').sound).toBe('horn');
    expect(screen.getByLabelText(it_('not2.muteTx'))).toBeChecked();
    fireEvent.click(screen.getByLabelText(it_('not2.muteTx')));
    expect(JSON.parse(localStorage.getItem('aurora-alerts') ?? '{}').muteWhileTx).toBe(false);
    expect(screen.getByRole('button', { name: it_('not2.test') })).toBeEnabled();
    expect(screen.getByRole('button', { name: it_('alert.button') })).toBeInTheDocument();
    localStorage.removeItem('aurora-alerts');
  });

  it('the display choices of the engine tab are real: different background, DX call and grid highlight, sound at the first message', async () => {
    render(<NativeNotifications />);
    const cyan = await screen.findByLabelText(it_('not2.cyan'));
    expect(cyan).toBeDisabled();
    fireEvent.click(screen.getByLabelText(it_('not2.diffBg')));
    expect(useNotifyView.getState().view.differentBackground).toBe(true);
    expect(cyan).toBeEnabled();
    expect(cyan).toBeChecked();
    fireEvent.click(screen.getByLabelText(it_('not2.yellow')));
    expect(useNotifyView.getState().view.diffYellow).toBe(true);
    expect(screen.getByLabelText(it_('not2.hlDxCall'))).toBeChecked();
    fireEvent.click(screen.getByLabelText(it_('not2.hlDxGrid')));
    fireEvent.click(screen.getByLabelText(it_('not2.soundFirst')));
    expect(useNotifyView.getState().view).toMatchObject({ highlightDxGrid: true, beepFirst: true });
  });

  it('an «also a sound» box turns on the sound of both message types of the row', async () => {
    render(<NativeNotifications />);
    await screen.findByLabelText(it_('not2.cqz'));
    const grid = document.querySelector('.jt-grid') as HTMLElement;
    const sounds = within(grid).getAllByLabelText(it_('not2.alsoSound'));
    // di base il paese suona (nuovo e su nuova banda); il clic li spegne tutti e due, un altro li riaccende
    expect(useNotify.getState().notify.sound).toMatchObject({ new_dxcc: true, new_dxcc_band: true });
    fireEvent.click(sounds[2]!);
    expect(useNotify.getState().notify.sound).toMatchObject({ new_dxcc: false, new_dxcc_band: false, new_call: false });
    fireEvent.click(sounds[2]!);
    expect(useNotify.getState().notify.sound).toMatchObject({ new_dxcc: true, new_dxcc_band: true });
  });
});

describe('General tab (as in JTDX)', () => {
  it('has the four sections, with the station editable and what Aurora cannot do switched off with the reason', async () => {
    render(<NativeGeneral />);
    expect(await screen.findByLabelText(it_('gen2.call'))).toHaveValue('IU3ABC');
    for (const legend of ['gen2.station', 'gen2.logFilter', 'gen2.screen', 'gen2.operation'] as const) expect(screen.getByText(it_(legend))).toBeInTheDocument();
    for (const label of ['gen2.region', 'gen2.type2', 'gen2.decode52', 'gen2.lastFreq', 'gen2.cwId', 'gen2.tune', 'gen2.cwInt'] as const) {
      expect(screen.getByLabelText(it_(label))).toBeDisabled();
    }
    expect(screen.getAllByText(it_('ens.r.cw_id')).length).toBeGreaterThan(0);
    expect(screen.getByLabelText(it_('gen2.watchdog'))).toHaveValue(6);
  });

  it('the grid filter needs the callsign filter; the options are sent with Apply', async () => {
    vi.mocked(api.putEngineSettings).mockResolvedValue(doc());
    render(<NativeGeneral />);
    const useGrid = await screen.findByLabelText(it_('gen2.useGrid'));
    expect(useGrid).toBeDisabled();
    fireEvent.click(screen.getByLabelText(it_('gen2.useCall')));
    expect(useGrid).toBeEnabled();
    fireEvent.change(screen.getByLabelText(it_('gen2.watchdog')), { target: { value: '9' } });
    fireEvent.click(screen.getByLabelText(it_('gen2.qsy')));
    fireEvent.click(screen.getByRole('button', { name: it_('ens.apply') }));
    await waitFor(() => expect(api.putEngineSettings).toHaveBeenCalledWith({ callsignLogFiltering: 'true', TxWatchdogTimer: '9', TxQSYAllowed: 'false' }));
  });

  it('the screen options are real and change the display preferences at once', async () => {
    render(<NativeGeneral />);
    fireEvent.click(await screen.findByLabelText(it_('gen2.blank')));
    fireEvent.click(screen.getByLabelText(it_('gen2.azimuth')));
    fireEvent.click(screen.getByLabelText(it_('gen2.showTx')));
    expect(usePrefs.getState().prefs).toMatchObject({ insertBlank: true, showAzimuth: true, showTx: false });
  });
});
