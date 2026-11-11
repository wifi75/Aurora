import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import type { ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../api/client', () => ({
  getStationInfo: vi.fn(),
  getEngineSettings: vi.fn(),
  putEngineSettings: vi.fn(),
  getControls: vi.fn(),
  postControl: vi.fn(),
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
import { STRINGS, translate } from '../i18n';
import { DecodeToggles } from '../components/DecodeToggles';
import { DECODER_ROWS, FTOL_MSK144_VALUES, FTOL_VALUES } from './DecoderOptions';
import { NativeDecodeOptions } from './JtdxTabs';

const it_ = (key: Parameters<typeof translate>[1]) => translate('it', key);

/** I valori di fabbrica di WSJT-X 3.3, come li manda il motore (`engine_settings::defaults`). */
const WSJTX: Record<string, string> = {
  FT8DecoderStart: 'normal',
  DecodeDepth: '3',
  SingleDecode: 'false',
  IncludeAveraging: 'false',
  AutoClearAvg: 'false',
  JT65AP: 'false',
  IncludeCorrelation: 'false',
  JT65Aggressive: '4',
  Ftol_JT65: '50',
  Ftol_JT9: '50',
  Ftol_MSK144: '50',
  Ftol_Q65: '50',
};

function doc(values: Record<string, string> = {}): EngineSettings {
  const items: EngineSettings['items'] = {};
  for (const k of Object.keys(WSJTX)) items[k] = { state: 'live', apply: 'now' };
  return { version: 1, file: 'settings-motore.json', values: { ...WSJTX, ...values }, items };
}

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  vi.mocked(api.getEngineSettings).mockReset().mockResolvedValue(doc());
  vi.mocked(api.putEngineSettings).mockReset();
});

describe('decoder options (Filters tab, «Decoding»)', () => {
  it('shows every option with the WSJT-X defaults', async () => {
    render(<NativeDecodeOptions />);
    expect(await screen.findByLabelText(it_('dec.depth'))).toHaveValue('3');
    expect([...(screen.getByLabelText(it_('dec.depth')) as HTMLSelectElement).options].map((o) => o.value)).toEqual(['1', '2', '3']);
    for (const label of ['dec.singleDecode', 'dec.includeAveraging', 'dec.autoClearAvg', 'dec.jt65Ap', 'dec.correlation'] as const) {
      expect(screen.getByLabelText(it_(label))).not.toBeChecked();
      expect(screen.getByLabelText(it_(label))).toBeEnabled();
    }
    expect(screen.getByLabelText(it_('dec.aggressive'))).toHaveValue(4);
    for (const label of ['dec.ftolJt65', 'dec.ftolJt9', 'dec.ftolMsk144', 'dec.ftolQ65'] as const) expect(screen.getByLabelText(it_(label))).toHaveValue('50');
    expect([...(screen.getByLabelText(it_('dec.ftolJt65')) as HTMLSelectElement).options].map((o) => Number(o.value))).toEqual([...FTOL_VALUES]);
    expect([...(screen.getByLabelText(it_('dec.ftolMsk144')) as HTMLSelectElement).options].map((o) => Number(o.value))).toEqual([...FTOL_MSK144_VALUES]);
    expect((screen.getByLabelText(it_('dec.ftolQ65')) as HTMLSelectElement).selectedOptions[0]?.textContent).toBe('50 Hz');
  });

  it('sends the changes with Apply', async () => {
    vi.mocked(api.putEngineSettings).mockResolvedValue(doc({ DecodeDepth: '1', IncludeAveraging: 'true', JT65Aggressive: '7', Ftol_JT65: '100', JT65AP: 'true' }));
    render(<NativeDecodeOptions />);
    fireEvent.change(await screen.findByLabelText(it_('dec.depth')), { target: { value: '1' } });
    fireEvent.click(screen.getByLabelText(it_('dec.includeAveraging')));
    fireEvent.click(screen.getByLabelText(it_('dec.jt65Ap')));
    fireEvent.change(screen.getByLabelText(it_('dec.aggressive')), { target: { value: '7' } });
    fireEvent.change(screen.getByLabelText(it_('dec.ftolJt65')), { target: { value: '100' } });
    fireEvent.click(screen.getByRole('button', { name: it_('ens.apply') }));
    await waitFor(() =>
      expect(api.putEngineSettings).toHaveBeenCalledWith({ DecodeDepth: '1', IncludeAveraging: 'true', JT65AP: 'true', JT65Aggressive: '7', Ftol_JT65: '100' }),
    );
  });

  it('every option has its label and its explanation in the six languages', () => {
    for (const { row, label } of DECODER_ROWS) {
      expect(STRINGS[label], label).toHaveLength(6);
      expect(`jxh.${row.key}` in STRINGS, row.key).toBe(true);
    }
  });
});

describe('quick decode toggles (next to the mode, like the WSJT-X Decode menu)', () => {
  it('show the state of the setting and change it at once', async () => {
    vi.mocked(api.putEngineSettings).mockResolvedValue(doc({ IncludeAveraging: 'true' }));
    render(<DecodeToggles keys={['IncludeAveraging', 'AutoClearAvg']} />);
    const avg = await screen.findByRole('button', { name: it_('dec.short.IncludeAveraging') });
    await waitFor(() => expect(avg).toBeEnabled());
    expect(avg).toHaveAttribute('aria-pressed', 'false');
    fireEvent.click(avg);
    await waitFor(() => expect(api.putEngineSettings).toHaveBeenCalledWith({ IncludeAveraging: 'true' }));
    await waitFor(() => expect(screen.getByRole('button', { name: it_('dec.short.IncludeAveraging') })).toHaveAttribute('aria-pressed', 'true'));
    expect(screen.getByRole('button', { name: it_('dec.short.AutoClearAvg') })).toHaveAttribute('aria-pressed', 'false');
  });

  it('are off when the setting is not available (not the own engine)', async () => {
    vi.mocked(api.getEngineSettings).mockRejectedValue(new Error('engine_settings_unavailable'));
    render(<DecodeToggles keys={['JT65AP', 'IncludeCorrelation']} />);
    const ap = await screen.findByRole('button', { name: it_('dec.short.JT65AP') });
    expect(ap).toBeDisabled();
    expect(screen.getByRole('button', { name: it_('dec.short.IncludeCorrelation') })).toBeDisabled();
  });
});
