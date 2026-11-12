import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor, within } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', async (original) => {
  const real = await original<typeof import('../api/client')>();
  return {
    ...real,
    getCat: vi.fn(),
    putCat: vi.fn(),
    connectCat: vi.fn(),
    disconnectCat: vi.fn(),
    catProfile: vi.fn(),
    getCatModels: vi.fn(),
    getCatPorts: vi.fn(),
    detectCat: vi.fn(),
    restoreHamlib: vi.fn(),
    searchCat4om: vi.fn(),
  };
});
import { ApiError, catProfile, connectCat, detectCat, disconnectCat, getCat, getCatModels, getCatPorts, putCat, restoreHamlib, searchCat4om, type CatSnapshot } from '../api/client';
import { DEFAULT_CAT } from '../lib/cat';
import { CatTab } from './CatTab';

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

const idle: CatSnapshot = {
  status: { backend: 'hamlib', state: 'disconnected', message: '', rigctld_found: true, rig: null },
  config: DEFAULT_CAT,
  profiles: [],
  active_profile: null,
};

const connected: CatSnapshot = {
  ...idle,
  status: {
    backend: 'hamlib',
    state: 'connected',
    message: '',
    rigctld_found: true,
    rig: { state: 'connected', message: '', frequency_hz: 14_074_000, mode: 'PKTUSB', ptt: false, split: null, failures: 0, reconnect_attempt: 0, ptt_blocked: true, updated_ms: 1 },
  },
};

const models = [
  { id: 1, manufacturer: 'Hamlib', model: 'Dummy', status: 'Stable' },
  { id: 1029, manufacturer: 'Yaesu', model: 'FT-2000', status: 'Stable' },
  { id: 3073, manufacturer: 'Icom', model: 'IC-7300', status: 'Stable' },
];

describe('CatTab', () => {
  beforeEach(() => {
    vi.mocked(getCat).mockReset().mockResolvedValue(idle);
    vi.mocked(putCat).mockReset().mockResolvedValue(idle);
    vi.mocked(connectCat).mockReset().mockResolvedValue(connected);
    vi.mocked(disconnectCat).mockReset().mockResolvedValue(idle);
    vi.mocked(catProfile).mockReset().mockResolvedValue(idle);
    vi.mocked(getCatModels).mockReset().mockResolvedValue(models);
    vi.mocked(getCatPorts).mockReset().mockResolvedValue([{ port: 'COM3', name: 'USB Serial Port', virtual: false }]);
    vi.mocked(detectCat).mockReset();
  });

  it('shows the state and warns when rigctld is missing', async () => {
    vi.mocked(getCat).mockResolvedValue({ ...idle, status: { ...idle.status, rigctld_found: false } });
    render(<CatTab />);
    expect(await screen.findByText('Non collegata')).toBeInTheDocument();
    expect(screen.getByText(/Manca rigctld/)).toBeInTheDocument();
  });

  it('restores Hamlib with one click when rigctld is missing (Windows)', async () => {
    vi.mocked(getCat).mockResolvedValue({ ...idle, status: { ...idle.status, rigctld_found: false } });
    vi.mocked(restoreHamlib).mockResolvedValue({ ...idle, status: { ...idle.status, rigctld_found: true } });
    render(<CatTab />);
    fireEvent.click(await screen.findByRole('button', { name: 'Ripristina Hamlib' }));
    expect(await screen.findByText(/Hamlib ripristinato/)).toBeInTheDocument();
    expect(restoreHamlib).toHaveBeenCalledTimes(1);
  });

  it('does not warn about rigctld when it is there, and Connect needs a radio first', async () => {
    render(<CatTab />);
    await screen.findByText('Non collegata');
    expect(screen.queryByText(/Manca rigctld/)).toBeNull();
    expect(screen.getByRole('button', { name: 'Collega' })).toBeDisabled();
  });

  it('picks a radio from the searchable list and applies the settings', async () => {
    render(<CatTab />);
    await screen.findByText('Non collegata');
    await screen.findByText('3 modelli');
    fireEvent.change(screen.getByLabelText('Cerca (marca o modello)'), { target: { value: 'yaesu 2000' } });
    const rig = screen.getByLabelText('Radio');
    expect(within(rig).getAllByRole('option').map((o) => o.textContent)).toEqual(['— scegli la radio —', 'Yaesu FT-2000']);
    fireEvent.change(rig, { target: { value: '1029' } });
    fireEvent.change(screen.getByLabelText('Porta'), { target: { value: 'COM3' } });
    fireEvent.change(screen.getByLabelText('Velocità (baud)'), { target: { value: '38400' } });
    fireEvent.change(screen.getByLabelText('Bit di stop'), { target: { value: '2' } });
    fireEvent.change(screen.getByLabelText('Controllo di flusso'), { target: { value: 'hardware' } });
    expect(await screen.findByText('Modifiche non ancora applicate.')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Applica' }));
    await waitFor(() => expect(putCat).toHaveBeenCalled());
    expect(vi.mocked(putCat).mock.calls[0]![0]).toMatchObject({ backend: 'hamlib', model: 1029, rig_name: 'Yaesu FT-2000', port: 'COM3', baud: 38400, stop_bits: 2, handshake: 'hardware', ptt: 'cat', split: 'none' });
    expect(await screen.findByText('Impostazioni salvate.')).toBeInTheDocument();
  });

  it('applies pending changes and then connects when Connect is pressed', async () => {
    vi.mocked(getCat).mockResolvedValue({ ...idle, config: { ...DEFAULT_CAT, model: 1029, rig_name: 'Yaesu FT-2000', port: 'COM3' } });
    render(<CatTab />);
    await screen.findByText('Non collegata');
    fireEvent.change(screen.getByLabelText('Velocità (baud)'), { target: { value: '19200' } });
    fireEvent.click(screen.getByRole('button', { name: 'Collega' }));
    await waitFor(() => expect(connectCat).toHaveBeenCalled());
    expect(putCat).toHaveBeenCalledTimes(1);
    expect(vi.mocked(putCat).mock.calls[0]![0].baud).toBe(19200);
  });

  it('shows frequency, mode and PTT when connected, and Disconnect works', async () => {
    vi.mocked(getCat).mockResolvedValue({ ...connected, config: { ...DEFAULT_CAT, model: 1029, port: 'COM3' } });
    render(<CatTab />);
    expect(await screen.findByText('Collegata')).toBeInTheDocument();
    expect(screen.getByText('14074.000 kHz')).toBeInTheDocument();
    expect(screen.getByText('PKTUSB')).toBeInTheDocument();
    expect(screen.getByText(/spento \(bloccato per qualche secondo/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Scollega' }));
    await waitFor(() => expect(disconnectCat).toHaveBeenCalled());
  });

  it('shows the translated error of the radio', async () => {
    vi.mocked(getCat).mockResolvedValue({
      ...idle,
      status: { ...idle.status, state: 'error', message: 'La radio non risponde: controlla accensione, cavo, porta.', rig: null },
    });
    render(<CatTab />);
    expect(await screen.findByText(/La radio non risponde/)).toBeInTheDocument();
    expect(screen.getByText('Errore')).toBeInTheDocument();
  });

  it('detects the radio passively and only fills the draft until Apply', async () => {
    vi.mocked(detectCat).mockResolvedValue({
      candidates: [
        {
          device_key: 'k',
          is_radio: true,
          protocol: 'newcat',
          rig_label: 'Yaesu FT-2000',
          rig_token: 'FT-2000',
          rig_model: 1029,
          alternatives: [],
          cat_port: 'COM5',
          other_ports: [],
          baud: 38400,
          civ_address: null,
          audio_input: 'USB Audio CODEC',
          audio_output: 'USB Audio CODEC',
          confidence: 90,
          evidence: 'il nome contiene il modello',
        },
      ],
      audio_devices: ['USB Audio CODEC'],
      ports_seen: 1,
    });
    render(<CatTab />);
    await screen.findByText('Non collegata');
    await screen.findByText('3 modelli');
    fireEvent.click(screen.getByRole('button', { name: 'Rileva la mia radio' }));
    expect(await screen.findByText('Yaesu FT-2000', { exact: false })).toBeInTheDocument();
    expect(screen.getByText('(fiducia 90%)')).toBeInTheDocument();
    expect(screen.getByText('Scheda audio: USB Audio CODEC')).toBeInTheDocument();
    expect(putCat).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Usa' }));
    expect(await screen.findByText('Scelto: controlla i valori e premi Applica.')).toBeInTheDocument();
    expect(putCat).not.toHaveBeenCalled();
    expect((screen.getByLabelText('Porta') as HTMLInputElement).value).toBe('COM5');
    expect((screen.getByLabelText('Radio') as HTMLSelectElement).value).toBe('1029');
    fireEvent.click(screen.getByRole('button', { name: 'Applica' }));
    await waitFor(() => expect(putCat).toHaveBeenCalled());
    expect(vi.mocked(putCat).mock.calls[0]![0]).toMatchObject({ model: 1029, port: 'COM5', baud: 38400 });
  });

  it('searches the Cat4OM groups and radios and puts the chosen one in the fields', async () => {
    vi.mocked(getCat).mockResolvedValue({ ...idle, status: { ...idle.status, backend: 'cat4om' }, config: { ...DEFAULT_CAT, backend: 'cat4om', cat4om_address: '127.0.0.1:5009' } });
    vi.mocked(searchCat4om).mockReset().mockResolvedValue({
      management: '127.0.0.1:17300',
      management_error: '',
      groups: [
        {
          id: 'group-1',
          name: 'Group 1',
          control: '127.0.0.1:5001',
          running: true,
          manual: false,
          audio: true,
          error: '',
          radios: [
            { id: 'kenwood/ts-990', name: 'TS-990', model: 'Kenwood TS-990', status: 'connected', claimed_by: null, audio_rx: true, audio_tx: true },
            { id: 'ft991', name: 'ft991', model: '', status: 'no_response', claimed_by: 'hamlib', audio_rx: null, audio_tx: null },
          ],
        },
        { id: 'locked', name: 'locked', control: '127.0.0.1:5002', running: true, manual: false, audio: null, error: 'Cat4OM chiede la chiave del gruppo: scrivila in Impostazioni › Stazione › Radio', radios: [] },
        { id: 'spare', name: 'spare', control: '', running: false, manual: false, audio: null, error: '', radios: [] },
      ],
    });
    render(<CatTab />);
    fireEvent.click(await screen.findByRole('button', { name: 'Cerca' }));
    const list = await screen.findByRole('region', { name: 'Gruppi e radio di Cat4OM' });
    expect(searchCat4om).toHaveBeenCalledWith({ management: '127.0.0.1:17300', address: '127.0.0.1:5009', key: '' });
    expect(within(list).getByText(/in funzione, porta Control 127\.0\.0\.1:5001 · audio in rete/)).toBeInTheDocument();
    expect(within(list).getByText(/collegata · audio in rete RX\/TX/)).toBeInTheDocument();
    expect(within(list).getByText(/non risponde · presa da hamlib/)).toBeInTheDocument();
    expect(within(list).getByText(/Cat4OM chiede la chiave del gruppo/)).toBeInTheDocument();
    expect(within(list).getByText(/fermo: Aurora lo avvia al collegamento/)).toBeInTheDocument();
    fireEvent.click(within(list).getAllByRole('button', { name: 'Usa' })[0]!);
    expect(await screen.findByText(/Gruppo «group-1» e radio «kenwood\/ts-990» scelti/)).toBeInTheDocument();
    expect(within(list).getByRole('button', { name: 'Scelta' })).toHaveAttribute('aria-pressed', 'true');
    fireEvent.click(screen.getByRole('button', { name: 'Applica' }));
    // il gruppo trovato dalla gestione si collega con la scoperta: la porta scritta a mano si toglie
    await waitFor(() => expect(putCat).toHaveBeenCalledWith(expect.objectContaining({ cat4om_group: 'group-1', cat4om_radio: 'kenwood/ts-990', cat4om_address: '' })));
  });

  it('says how to get the Cat4OM network audio and why it does not work', async () => {
    const audio_error = "Cat4OM: un altro programma sta gia' mandando l'audio di trasmissione a questa radio";
    vi.mocked(getCat).mockResolvedValue({
      ...idle,
      status: { ...idle.status, backend: 'cat4om', cat4om: { management: '127.0.0.1:17300', control: '127.0.0.1:5001', discovered: true, group: 'group-1', radio: 'run', role: 'slave', audio_error } },
      config: { ...DEFAULT_CAT, backend: 'cat4om' },
    });
    render(<CatTab />);
    expect(await screen.findByText(/Audio in rete: Cat4OM: un altro programma sta già mandando l’audio di trasmissione/)).toBeInTheDocument();
    expect(screen.getByText(/scegli «Cat4OM \(audio in rete\)» come ingresso e come uscita/)).toBeInTheDocument();
  });

  it('says why the Cat4OM search failed', async () => {
    vi.mocked(getCat).mockResolvedValue({ ...idle, status: { ...idle.status, backend: 'cat4om' }, config: { ...DEFAULT_CAT, backend: 'cat4om' } });
    vi.mocked(searchCat4om)
      .mockReset()
      .mockRejectedValue(new ApiError(502, 'cat4om_search', undefined, 'Gestione di Cat4OM non raggiungibile su 127.0.0.1:17300: avvia Log4OM e Cat4OM; da un altro PC abilita la rete in Settings → SERVER (rifiutato)'));
    render(<CatTab />);
    fireEvent.click(await screen.findByRole('button', { name: 'Cerca' }));
    expect(await screen.findByText(/Gestione di Cat4OM non raggiungibile su 127\.0\.0\.1:17300/)).toBeInTheDocument();
    expect(screen.queryByRole('region', { name: 'Gruppi e radio di Cat4OM' })).toBeNull();
  });

  it('says so when nothing is found', async () => {
    vi.mocked(detectCat).mockResolvedValue({ candidates: [], audio_devices: [], ports_seen: 0 });
    render(<CatTab />);
    await screen.findByText('Non collegata');
    fireEvent.click(screen.getByRole('button', { name: 'Rileva la mia radio' }));
    expect(await screen.findByText(/Nessuna porta seriale trovata/)).toBeInTheDocument();
  });

  it('saves, loads, creates and deletes profiles', async () => {
    const withProfile: CatSnapshot = { ...idle, profiles: [{ name: 'FT-2000 casa', config: { ...DEFAULT_CAT, model: 1029 } }], active_profile: null };
    vi.mocked(getCat).mockResolvedValue(withProfile);
    render(<CatTab />);
    await screen.findByText('Non collegata');
    fireEvent.change(screen.getByLabelText('Nome del profilo'), { target: { value: 'Seconda radio' } });
    fireEvent.click(screen.getByRole('button', { name: 'Salva' }));
    await waitFor(() => expect(catProfile).toHaveBeenCalledWith('save', 'Seconda radio'));
    expect(await screen.findByText('Profilo salvato.')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Profili salvati'), { target: { value: 'FT-2000 casa' } });
    fireEvent.click(screen.getByRole('button', { name: 'Carica' }));
    await waitFor(() => expect(catProfile).toHaveBeenCalledWith('load', 'FT-2000 casa'));
    expect(await screen.findByText(/La radio è scollegata: premi Collega/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Elimina' }));
    await waitFor(() => expect(catProfile).toHaveBeenCalledWith('delete', 'FT-2000 casa'));
  });

  it('explains that changes are made from the PC only', async () => {
    vi.mocked(getCat).mockResolvedValue({ ...idle, config: { ...DEFAULT_CAT, model: 1029, port: 'COM3' } });
    vi.mocked(connectCat).mockRejectedValue(new ApiError(403, 'local_only'));
    render(<CatTab />);
    await screen.findByText('Non collegata');
    fireEvent.click(screen.getByRole('button', { name: 'Collega' }));
    expect(await screen.findByText(/solo dal PC dove gira Aurora/)).toBeInTheDocument();
  });

  it('shows the detail the core sends when connecting fails', async () => {
    vi.mocked(getCat).mockResolvedValue({ ...idle, config: { ...DEFAULT_CAT, model: 1029, port: 'COM3' } });
    vi.mocked(connectCat).mockRejectedValue(new ApiError(409, 'rigctld_missing', undefined, 'Manca rigctld (Hamlib).'));
    render(<CatTab />);
    await screen.findByText('Non collegata');
    fireEvent.click(screen.getByRole('button', { name: 'Collega' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Manca rigctld (Hamlib).');
  });

  it('hides the Hamlib fields with OmniRig and keeps split and mode', async () => {
    vi.mocked(getCat).mockResolvedValue({ ...idle, config: { ...DEFAULT_CAT, backend: 'omnirig' }, status: { ...idle.status, backend: 'omnirig' } });
    render(<CatTab />);
    await screen.findByText('Non collegata');
    expect(screen.queryByLabelText('Porta')).toBeNull();
    expect(screen.queryByLabelText('Radio')).toBeNull();
    expect(screen.queryByLabelText('Metodo del PTT')).toBeNull();
    expect(screen.getByLabelText('Split')).toBeInTheDocument();
    expect(screen.getByLabelText('Modo scritto nella radio')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Rileva la mia radio' })).toBeNull();
  });

  it('shows the PTT port only for DTR or RTS and the CI-V address only for Icom', async () => {
    vi.mocked(getCat).mockResolvedValue({ ...idle, config: { ...DEFAULT_CAT, model: 3073, rig_name: 'Icom IC-7300', port: 'COM4' } });
    render(<CatTab />);
    await screen.findByText('Non collegata');
    await screen.findByText('3 modelli');
    expect(screen.queryByLabelText(/Porta del PTT/)).toBeNull();
    expect(screen.getByLabelText('Indirizzo CI-V (Icom)')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Metodo del PTT'), { target: { value: 'rts' } });
    expect(screen.getByLabelText(/Porta del PTT/)).toBeInTheDocument();
  });

  it('refuses to apply values the core would refuse', async () => {
    render(<CatTab />);
    await screen.findByText('Non collegata');
    fireEvent.change(screen.getByLabelText('Lettura della radio ogni (s)'), { target: { value: '0' } });
    expect(await screen.findByText(/Un valore non va/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Applica' })).toBeDisabled();
  });

  it('shows where to connect the other programs only when sharing is on and the radio is connected', async () => {
    vi.mocked(getCat).mockResolvedValue({ ...connected, config: { ...DEFAULT_CAT, model: 1029, share_enabled: true }, status: { ...connected.status, share: { host: '127.0.0.1', port: 4532 } } });
    render(<CatTab />);
    expect(await screen.findByText(/127\.0\.0\.1:4532/)).toBeInTheDocument();
    expect(screen.getByLabelText('Porta di condivisione')).toHaveValue(4532);
    fireEvent.change(screen.getByLabelText('Porta di condivisione'), { target: { value: '80' } });
    expect(await screen.findByText(/Un valore non va/)).toBeInTheDocument();
  });

  it('offers sharing even before connecting, without the sharing port until it is on', async () => {
    render(<CatTab />);
    await screen.findByText('Non collegata');
    expect(screen.getByText(/Condividi la radio con altri programmi/)).toBeInTheDocument();
    expect(screen.queryByLabelText('Porta di condivisione')).toBeNull();
  });
});
