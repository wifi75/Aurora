import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError, type WefaxSnapshot } from '../../api/client';
import { Controls } from './Controls';

const api = vi.hoisted(() => ({ postWefaxFile: vi.fn(), stopWefaxFile: vi.fn(), putWefax: vi.fn(), wefaxRx: vi.fn() }));

vi.mock('../../api/client', async (importOriginal) => ({ ...(await importOriginal<typeof import('../../api/client')>()), ...api }));

function snap(over: Partial<WefaxSnapshot['rx']> = {}): WefaxSnapshot {
  return {
    rx: { listening: true, receiving: false, state: 'apt', ioc: 576, lpm: 120, width: 1809, rows: 0, sync: 0, level_db: -40, last_image: null, ...over },
    settings: {
      ioc: 576,
      lpm_576: 120,
      lpm_288: 60,
      filter: 'narrow',
      manual: false,
      auto_lpm: true,
      auto_center: true,
      auto_save: true,
      auto_receive: false,
      center_hz: 1900,
      shift_hz: 800,
      slant_ppm: 0,
      max_rows: 2000,
      correlation: 0.05,
      folder: '',
      afc: true,
      squelch: false,
      squelch_level: 30,
    },
    folder: '',
    iocs: [{ ioc: 576, width: 1809, apt_start_hz: 300, default_lpm: 120 }],
    lpm_values: [60, 90, 120, 240],
    filters: ['narrow', 'medium', 'wide'],
  };
}

const wav = (name = 'cartina.wav', size = 10) => new File([new Uint8Array(size)], name, { type: 'audio/wav' });

beforeEach(() => {
  for (const f of Object.values(api)) f.mockReset();
  api.postWefaxFile.mockResolvedValue(undefined);
  api.stopWefaxFile.mockResolvedValue(undefined);
});

describe('Controls: decodifica da file', () => {
  it('manda il file scelto e non mostra errori se va bene', async () => {
    const { container } = render(<Controls snap={snap()} onChanged={() => undefined} />);
    const input = container.querySelector('input[type="file"]') as HTMLInputElement;
    fireEvent.change(input, { target: { files: [wav()] } });
    await waitFor(() => expect(api.postWefaxFile).toHaveBeenCalledTimes(1));
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('dice con chiarezza che il file non e valido, che e occupato o che e troppo grande', async () => {
    const { container } = render(<Controls snap={snap()} onChanged={() => undefined} />);
    const input = container.querySelector('input[type="file"]') as HTMLInputElement;
    api.postWefaxFile.mockRejectedValueOnce(new ApiError(400, 'bad_audio'));
    fireEvent.change(input, { target: { files: [wav('non-e-un-wav.txt')] } });
    expect(await screen.findByRole('alert')).toHaveTextContent('WAV');
    api.postWefaxFile.mockRejectedValueOnce(new ApiError(409, 'busy'));
    fireEvent.change(input, { target: { files: [wav()] } });
    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('già in decodifica'));
    // oltre 200 MB non si manda nemmeno
    api.postWefaxFile.mockClear();
    fireEvent.change(input, { target: { files: [{ ...wav(), size: 201 * 1024 * 1024 }] } });
    expect(screen.getByRole('alert')).toHaveTextContent('200 MB');
    expect(api.postWefaxFile).not.toHaveBeenCalled();
  });

  it('durante la decodifica mostra la percentuale e il pulsante per fermarla, e il pulsante del file non c e', () => {
    const { container } = render(<Controls snap={snap({ decoding_file: true, file_percent: 42 })} onChanged={() => undefined} />);
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '42');
    expect(screen.getByText('42 %')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Ferma la decodifica' }));
    expect(api.stopWefaxFile).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole('button', { name: 'Decodifica da file…' })).toBeNull();
    expect(container.querySelector('input[type="file"]')).not.toBeNull();
  });
});
