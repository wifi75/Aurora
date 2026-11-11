import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render as rtlRender, screen, waitFor } from '@testing-library/react';
import { useState, type ReactElement } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { SstvSnapshot } from '../api/client';

vi.mock('../api/client', () => ({ getSstv: vi.fn(), getSstvImages: vi.fn(), putSstv: vi.fn() }));
import { getSstv, getSstvImages, putSstv } from '../api/client';
import { usePrefs } from '../lib/prefs';
import { collectEntries } from '../lib/settingsSearch';
import { SessionDraft, type SharedDraft } from './session';
import { SSTV_DRAFT, SstvTab, sstvDraftToSettings, type SstvDraft } from './SstvTab';

const SNAP: SstvSnapshot = {
  rx: { listening: true, receiving: false, mode: null, width: 320, height: 256, rows: 0, sync: 0, offset_hz: 0, level_db: -40, last_image: null },
  tx: { state: 'idle', mode: null, progress: 0, elapsed_s: 0, total_s: 0, message: null },
  settings: { default_mode: 'm1', auto_save: true, save_tx: false, auto_receive: false, lock_mode: null, folder: '', tx_folder: '', rx_offset_hz: 0, tx_offset_hz: 0, slant_ppm: 0 },
  folder: 'C:\\Aurora\\history',
  can_transmit: true,
  modes: [
    { id: 'm1', name: 'Martin 1', width: 320, height: 256, vis: 44, seconds: 114, color: 'rgb' },
    { id: 's1', name: 'Scottie 1', width: 320, height: 256, vis: 60, seconds: 110, color: 'rgb' },
  ],
};

function render(ui: ReactElement) {
  return rtlRender(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{ui}</QueryClientProvider>);
}

function sharedDraft() {
  const setDraftOf = vi.fn<(key: string, value: unknown) => void>();
  const draft: SharedDraft = { changes: {}, set: () => undefined, clear: () => undefined, drafts: {}, setDraftOf };
  return { draft, setDraftOf };
}

beforeEach(() => {
  localStorage.clear();
  usePrefs.getState().reset();
  vi.mocked(getSstv).mockReset().mockResolvedValue(SNAP);
  vi.mocked(getSstvImages).mockReset().mockResolvedValue([]);
  vi.mocked(putSstv).mockReset().mockResolvedValue(SNAP);
});

describe('SstvTab', () => {
  it('shows the receive settings in normal cards, without the old pill wrapper', async () => {
    const { container } = render(<SstvTab part="rx" />);
    expect(await screen.findByText('Ricezione')).toBeInTheDocument();
    expect(screen.getByText('Orologio della scheda audio')).toBeInTheDocument();
    expect(screen.getByText('Visore')).toBeInTheDocument();
    expect(screen.getByLabelText('Solo il modo')).toBeInTheDocument();
    expect(screen.getByLabelText('Ascolto acceso all’avvio di Aurora')).not.toBeChecked();
    expect(screen.getByRole('button', { name: 'Calibra la scheda audio' })).toBeDisabled();
    expect(screen.getByText(/QSSTV/)).toHaveClass('sx-credits');
    // la scheda non ha un pulsante «Salva» suo: si salva con quello in basso della finestra
    expect(screen.queryByRole('button', { name: 'Salva' })).toBeNull();
    expect(container.querySelector('.sstv-tab')).toBeNull();
    expect(container.querySelectorAll('.sx-group').length).toBe(3);
  });

  it('switches the smooth enlargement at once (shared with the SSTV page)', async () => {
    render(<SstvTab part="rx" />);
    fireEvent.click(await screen.findByLabelText('Ingrandimento morbido'));
    expect(usePrefs.getState().prefs.sstvSmooth).toBe(true);
  });

  it('puts the changes in the window draft instead of saving them', async () => {
    const { draft, setDraftOf } = sharedDraft();
    render(
      <SessionDraft.Provider value={draft}>
        <SstvTab part="tx" />
      </SessionDraft.Provider>,
    );
    expect(await screen.findByText('Trasmissione')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Spostamento toni in trasmissione (Hz)'), { target: { value: '-15' } });
    expect(setDraftOf).toHaveBeenCalledWith(SSTV_DRAFT, expect.objectContaining({ tx_offset_hz: '-15', default_mode: 'm1' }));
    expect(putSstv).not.toHaveBeenCalled();
  });

  it('drops the draft when the value goes back to the saved one', async () => {
    const calls: unknown[] = [];
    function Harness() {
      const [drafts, setDrafts] = useState<Record<string, unknown>>({});
      const value: SharedDraft = {
        changes: {},
        set: () => undefined,
        clear: () => undefined,
        drafts,
        setDraftOf: (key, v) => {
          calls.push(v);
          setDrafts((d) => {
            const next = { ...d };
            if (v === undefined) delete next[key];
            else next[key] = v;
            return next;
          });
        },
      };
      return (
        <SessionDraft.Provider value={value}>
          <SstvTab part="images" />
        </SessionDraft.Provider>
      );
    }
    render(<Harness />);
    const box = await screen.findByLabelText('Salva ogni immagine ricevuta');
    fireEvent.click(box);
    expect(box).not.toBeChecked();
    expect(calls[0]).toEqual(expect.objectContaining({ auto_save: false }));
    fireEvent.click(box);
    expect(box).toBeChecked();
    expect(calls[1]).toBeUndefined();
  });

  it('shows the folders with their explanation under the field', async () => {
    render(<SstvTab part="images" />);
    expect(await screen.findByText('Cartelle')).toBeInTheDocument();
    expect(screen.getByLabelText('Cartella delle immagini')).toHaveAttribute('placeholder', 'C:\\Aurora\\history');
    expect(screen.getByText(/la cartella history dentro la cartella di Aurora/)).toHaveClass('sx-below');
  });

  it('saves by itself outside the settings window', async () => {
    render(<SstvTab part="tx" />);
    fireEvent.change(await screen.findByLabelText('Spostamento toni in trasmissione (Hz)'), { target: { value: '10' } });
    await waitFor(() => expect(putSstv).toHaveBeenCalledWith(expect.objectContaining({ tx_offset_hz: 10 })));
  });
});

describe('settings search', () => {
  it('finds the SSTV options by their short names', async () => {
    const { container } = render(
      <>
        <div data-search-tab="sstv">
          <SstvTab part="rx" />
        </div>
        <div data-search-tab="sstvimages">
          <SstvTab part="images" />
        </div>
      </>,
    );
    await screen.findByText('Cartelle');
    const texts = [...container.querySelectorAll<HTMLElement>('[data-search-tab]')].flatMap((el) => collectEntries(el.dataset.searchTab ?? '', el));
    expect(texts).toEqual(expect.arrayContaining([{ tab: 'sstv', text: 'Ingrandimento morbido' }, { tab: 'sstv', text: 'Correzione inclinazione (ppm)' }, { tab: 'sstvimages', text: 'Cartella delle immagini' }]));
    // le spiegazioni sotto le cartelle non entrano nel nome della voce
    expect(texts.some((e) => e.text.includes('cartella history'))).toBe(false);
  });
});

describe('sstvDraftToSettings', () => {
  const base: SstvDraft = { ...SNAP.settings, rx_offset_hz: '0', tx_offset_hz: '0', slant_ppm: '0' };
  it('turns the typed numbers into settings and refuses the ones out of range', () => {
    expect(sstvDraftToSettings({ ...base, rx_offset_hz: '-20', slant_ppm: '150' })).toEqual({ ...SNAP.settings, rx_offset_hz: -20, slant_ppm: 150 });
    expect(sstvDraftToSettings({ ...base, rx_offset_hz: '-' })).toBeNull();
    expect(sstvDraftToSettings({ ...base, tx_offset_hz: '201' })).toBeNull();
    expect(sstvDraftToSettings({ ...base, slant_ppm: '1.5' })).toBeNull();
  });
});
