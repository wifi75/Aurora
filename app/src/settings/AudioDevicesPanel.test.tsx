import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor, within } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', async (original) => {
  const real = await original<typeof import('../api/client')>();
  return { ...real, getAudioDevices: vi.fn(), putAudioSettings: vi.fn() };
});
import { ApiError, getAudioDevices, putAudioSettings, type AudioDevices, type AudioResult } from '../api/client';
import { AudioDevicesPanel } from './AudioDevicesPanel';
import { useEngineOs } from '../i18n';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

const devices: AudioDevices = {
  inputs: [
    { name: 'Microfono USB', channels: 2, is_default: true },
    { name: 'Line In', channels: 1, is_default: false },
  ],
  outputs: [{ name: 'Altoparlanti USB', channels: 2, is_default: true }],
  input_in_use: 'Microfono USB',
  output_in_use: 'Altoparlanti USB',
  output_pending: null,
  default_save_dir: 'D:/dati/save',
  system_default: 'Default',
  values: { input: 'Microfono USB', output: '', input_channel: 'Left', output_channel: 'Mono', save_dir: '', save_mode: '0', save_keep_days: '7' },
};

const part = (status: AudioResult['input']['status'], in_use: string | null = null, extra: Partial<AudioResult['input']> = {}): AudioResult['input'] => ({ status, in_use, restored: false, message: null, ...extra });

describe('AudioDevicesPanel', () => {
  beforeEach(() => {
    useEngineOs.setState({ os: 'windows' });
    vi.mocked(getAudioDevices).mockReset().mockResolvedValue(devices);
    vi.mocked(putAudioSettings).mockReset();
  });

  it('lists the devices with their channels, marks the one in use and shows the saved values', async () => {
    render(<AudioDevicesPanel />);
    const input = (await screen.findByLabelText('Ingresso audio')) as HTMLSelectElement;
    expect(input.value).toBe('Microfono USB');
    expect(screen.getByRole('option', { name: 'Microfono USB · stereo · in uso' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'Line In · mono' })).toBeInTheDocument();
    expect(screen.getAllByRole('option', { name: 'Predefinito del sistema (Microfono USB)' }).length).toBe(1);
    expect((screen.getByLabelText('Canale d’ingresso') as HTMLSelectElement).value).toBe('Left');
    expect(screen.getByText('In uso adesso: Microfono USB')).toBeInTheDocument();
    expect(screen.getByText('Trasmissione su: Altoparlanti USB')).toBeInTheDocument();
    expect((screen.getByLabelText('Uscita audio (trasmissione)') as HTMLSelectElement).value).toBe('Default');
    expect(screen.getByPlaceholderText('Predefinita: D:/dati/save')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Applica' })).toBeDisabled();
  });

  it('a mono device turns the channel back to Mono and disables Left and Right', async () => {
    render(<AudioDevicesPanel />);
    fireEvent.change(await screen.findByLabelText('Ingresso audio'), { target: { value: 'Line In' } });
    expect((screen.getByLabelText('Canale d’ingresso') as HTMLSelectElement).value).toBe('Mono');
    const channel = within(screen.getByLabelText('Canale d’ingresso'));
    expect(channel.getByRole('option', { name: 'Sinistro' })).toBeDisabled();
    expect(channel.getByRole('option', { name: 'Destro' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Applica' })).toBeEnabled();
  });

  it('sends only what changed and tells that it was applied', async () => {
    vi.mocked(putAudioSettings).mockResolvedValue({ ok: true, input: part('applied', 'Line In'), output: part('unchanged'), save: part('applied'), values: { ...devices.values, input: 'Line In', input_channel: 'Mono', save_mode: '1' } });
    render(<AudioDevicesPanel />);
    fireEvent.change(await screen.findByLabelText('Ingresso audio'), { target: { value: 'Line In' } });
    fireEvent.change(screen.getByLabelText('Salvataggio dei file audio'), { target: { value: '1' } });
    fireEvent.click(screen.getByRole('button', { name: 'Applica' }));
    await waitFor(() => expect(putAudioSettings).toHaveBeenCalledWith({ input: 'Line In', input_channel: 'Mono', save_mode: 1 }));
    expect(await screen.findByText('Ingresso: applicato, in uso Line In.')).toBeInTheDocument();
    expect(screen.getByText('Impostazioni audio applicate e salvate.')).toBeInTheDocument();
  });

  it('sends the days to keep the saved files as a number and ignores an impossible value', async () => {
    vi.mocked(putAudioSettings).mockResolvedValue({ ok: true, input: part('unchanged'), output: part('unchanged'), save: part('applied'), values: { ...devices.values, save_keep_days: '30' } });
    render(<AudioDevicesPanel />);
    const keep = (await screen.findByLabelText('Tieni i file salvati (giorni)')) as HTMLInputElement;
    expect(keep.value).toBe('7');
    fireEvent.change(keep, { target: { value: '5000' } });
    expect(screen.getByRole('button', { name: 'Applica' })).toBeEnabled();
    fireEvent.change(keep, { target: { value: '30' } });
    fireEvent.click(screen.getByRole('button', { name: 'Applica' }));
    await waitFor(() => expect(putAudioSettings).toHaveBeenCalledWith({ save_keep_days: 30 }));
  });

  it('says clearly when a device did not open and which one stays in use', async () => {
    vi.mocked(putAudioSettings).mockResolvedValue({
      ok: false,
      input: part('failed', 'Microfono USB', { restored: true, message: 'ingresso audio non aperto' }),
      output: part('deferred', 'Altoparlanti USB'),
      save: part('unchanged'),
      values: devices.values,
    });
    render(<AudioDevicesPanel />);
    fireEvent.change(await screen.findByLabelText('Ingresso audio'), { target: { value: 'Line In' } });
    fireEvent.change(screen.getByLabelText('Uscita audio (trasmissione)'), { target: { value: 'Altoparlanti USB' } });
    fireEvent.click(screen.getByRole('button', { name: 'Applica' }));
    expect(await screen.findByText(/Ingresso: non applicato \(ingresso audio non aperto\)\. La scelta non è stata salvata\. Resta in uso Microfono USB\./)).toBeInTheDocument();
    expect(screen.getByText(/Uscita: Altoparlanti USB è aperta e vale dalla prossima trasmissione/)).toBeInTheDocument();
    expect(screen.queryByText('Impostazioni audio applicate e salvate.')).not.toBeInTheDocument();
  });

  it('shows the error of the request and refreshes the list on demand', async () => {
    vi.mocked(putAudioSettings).mockRejectedValue(new ApiError(403, 'local_only'));
    render(<AudioDevicesPanel />);
    fireEvent.change(await screen.findByLabelText('Ingresso audio'), { target: { value: 'Line In' } });
    fireEvent.click(screen.getByRole('button', { name: 'Applica' }));
    expect(await screen.findByText('Solo dal PC di casa.')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Aggiorna elenco' }));
    await waitFor(() => expect(getAudioDevices).toHaveBeenCalledTimes(2));
  });

  it('shows why the input or the output did not open, as the system says it', async () => {
    vi.mocked(getAudioDevices).mockResolvedValue({
      ...devices,
      input_in_use: null,
      output_in_use: null,
      input_error: "ingresso audio non aperto (occupato da un'altra applicazione?): PCM2901: 48000 Hz, 2 canali, I16, buffer 1920 campioni: Device busy",
      output_error: 'PCM2901: uscita non aperta: Device busy',
    });
    render(<AudioDevicesPanel />);
    expect(await screen.findByText(/L’ingresso non si è aperto: .*48000 Hz, 2 canali, I16.*Device busy/)).toBeInTheDocument();
    expect(screen.getByText('L’uscita non si è aperta: PCM2901: uscita non aperta: Device busy')).toBeInTheDocument();
  });

  it('with the engine on Linux no text names Windows: the volume is the one of the system mixer', async () => {
    useEngineOs.setState({ os: 'linux' });
    vi.mocked(getAudioDevices).mockResolvedValue({
      ...devices,
      inputs: [{ name: 'PCM2901 Audio Codec Analog Stereo', channels: 2, is_default: true }],
      outputs: [{ name: 'PCM2901 Audio Codec Analog Stereo', channels: 2, is_default: true }],
      input_in_use: 'PCM2901 Audio Codec Analog Stereo',
      output_in_use: 'PCM2901 Audio Codec Analog Stereo',
    });
    const { container } = render(<AudioDevicesPanel />);
    // i canali sono quelli con cui il dispositivo si apre (stereo), non i 31 dichiarati dal server audio
    expect((await screen.findAllByRole('option', { name: 'PCM2901 Audio Codec Analog Stereo · stereo · in uso' })).length).toBe(2);
    // il volume non si legge (nelle prove come su Linux per l'ingresso): il testo manda al mixer del sistema
    expect((await screen.findAllByText('Il volume di questo dispositivo si regola dal mixer audio del sistema.')).length).toBeGreaterThan(0);
    expect(container.textContent).not.toMatch(/Windows/);
    expect(container.innerHTML).not.toMatch(/Windows/);
  });

  it('without a running input it says so and offers the saved choice', async () => {
    vi.mocked(getAudioDevices).mockResolvedValue({ ...devices, input_in_use: null, values: { ...devices.values, input: '' } });
    render(<AudioDevicesPanel />);
    expect(await screen.findByText(/Nessun ingresso attivo/)).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'Come all’avvio (nessun ingresso scelto)' })).toBeInTheDocument();
  });
});
