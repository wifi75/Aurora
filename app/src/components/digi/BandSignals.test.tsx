import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../api/client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../api/client')>()),
  clearDigiBand: vi.fn(() => Promise.resolve()),
  getDigiBand: vi.fn(() => Promise.resolve([])),
}));
import { clearDigiBand, type DigiSnapshot } from '../../api/client';
import type { DigiSignal } from '../../api/schemas';
import { emitDigi } from '../../lib/digiBus';
import { getBandSignals, waterfallLabels } from '../../lib/digiBand';
import { digiWithDefaults } from '../../lib/digiMacroEdit';
import { BAND_HEIGHT_KEY, BandSignals, bandHeightLimits } from './BandSignals';

function signal(id: number, text: string, stale = false): DigiSignal {
  return { id, freq_hz: 700 + id * 100, wpm: 22, snr_db: 12, text, unsure: [], calls: [{ call: 'IU3ABC', role: 'cq', verified: true, stale }], cq: true };
}

function panel(over: Partial<Parameters<typeof BandSignals>[0]> = {}) {
  return (
    <QueryClientProvider client={new QueryClient()}>
      <BandSignals signals={[signal(1, 'CQ DE IU3ABC')]} showWpm dialHz={0} rxHz={0} emptyText="vuoto" validation="normal" onValidation={() => {}} onTune={() => {}} onCall={() => {}} {...over} />
    </QueryClientProvider>
  );
}

beforeEach(() => {
  localStorage.clear();
  vi.mocked(clearDigiBand).mockClear();
});

describe('pannello Segnali: la convalida dei nominativi', () => {
  it('le impostazioni di tutta la banda non si perdono rileggendole dal core (la tendina tornava sempre «Normale»)', () => {
    const snap = { rx: {}, tx: {}, settings: { mode: 'cw', band_validation: 'strict', band_cw: false, band_spots: false, band_spots_cq_only: true } } as unknown as DigiSnapshot;
    const s = digiWithDefaults(snap).settings;
    expect(s.band_validation).toBe('strict');
    expect(s.band_cw).toBe(false);
    expect(s.band_spots).toBe(false);
    expect(s.band_spots_cq_only).toBe(true);
    // i valori di base degli altri campi restano
    expect(s.rx_freq_hz).toBe(1000);
  });

  it('la scelta parte, si vede subito e il menu resta aperto mentre l’elenco si aggiorna', () => {
    const onValidation = vi.fn();
    const { rerender } = render(panel({ onValidation }));
    fireEvent.click(screen.getByRole('button', { name: /Convalida: Normale/ }));
    expect(screen.getByRole('menu')).toBeTruthy();
    // un elenco nuovo dal core (due al secondo): il menu non si chiude
    rerender(panel({ onValidation, signals: [signal(1, 'CQ DE IU3ABC K'), signal(2, 'TEST')] }));
    expect(screen.getByRole('menu')).toBeTruthy();
    fireEvent.click(screen.getByRole('menuitemradio', { name: /Severa/ }));
    expect(onValidation).toHaveBeenCalledWith('strict');
    expect(screen.queryByRole('menu')).toBeNull();
    // le impostazioni salvate non sono ancora tornate (`validation` e' ancora «normal»): si vede gia' la scelta
    rerender(panel({ onValidation, signals: [signal(1, 'CQ DE IU3ABC K')] }));
    expect(screen.getByRole('button', { name: /Convalida: Severa/ })).toBeTruthy();
    // tornate: resta
    rerender(panel({ onValidation, validation: 'strict' }));
    expect(screen.getByRole('button', { name: /Convalida: Severa/ })).toBeTruthy();
  });

  it('se il salvataggio non torna, dopo un po’ si rivede il valore salvato', () => {
    vi.useFakeTimers();
    try {
      render(panel());
      fireEvent.click(screen.getByRole('button', { name: /Convalida: Normale/ }));
      fireEvent.click(screen.getByRole('menuitemradio', { name: /Massima/ }));
      expect(screen.getByRole('button', { name: /Convalida: Massima/ })).toBeTruthy();
      act(() => {
        vi.advanceTimersByTime(5000);
      });
      expect(screen.getByRole('button', { name: /Convalida: Normale/ })).toBeTruthy();
    } finally {
      vi.useRealTimers();
    }
  });
});

describe('pannello Segnali: pulizia, etichette e altezza', () => {
  it('«Pulisci» svuota l’elenco e chiede al core di ripartire da zero', () => {
    act(() => emitDigi({ type: 'digi', kind: 'band', mode: 'cw', signals: [signal(1, 'CQ DE IU3ABC')] }));
    expect(getBandSignals()).toHaveLength(1);
    render(panel());
    fireEvent.click(screen.getByRole('button', { name: 'Pulisci' }));
    expect(getBandSignals()).toHaveLength(0);
    expect(clearDigiBand).toHaveBeenCalledTimes(1);
  });

  it('sulla cascata restano solo i nominativi ancora sentiti', () => {
    expect(waterfallLabels([signal(1, 'CQ DE IU3ABC')])).toEqual([{ hz: 800, text: 'IU3ABC', cq: true, sure: true }]);
    expect(waterfallLabels([signal(1, 'E E T', true)])).toEqual([]);
  });

  it('l’altezza: almeno tre righe, al massimo meta’ della colonna, ricordata', () => {
    expect(bandHeightLimits(20, 24, 600)).toEqual({ min: 84, max: 370 });
    // senza misure (pannello vuoto): valori di riserva
    expect(bandHeightLimits(0, 0, 0)).toEqual({ min: 85, max: 400 });
    // colonna bassissima: il minimo vince
    expect(bandHeightLimits(20, 24, 100)).toEqual({ min: 84, max: 84 });
    localStorage.setItem(BAND_HEIGHT_KEY, '210');
    const { container } = render(panel());
    expect(container.querySelector<HTMLElement>('.digi-band-scroll')?.style.height).toBe('210px');
    expect(container.querySelector('.digi-band-grip')).toBeTruthy();
  });
});
