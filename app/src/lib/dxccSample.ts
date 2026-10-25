import type { DxccLabels, DxccMatrix } from './dxccCard';
import { DXCC_BANDS } from './dxccPrefs';

/** Dati d'esempio per l'anteprima nelle impostazioni: nessun dato del log vero. */
export function sampleMatrix(): DxccMatrix {
  const totals: DxccMatrix['totals'] = {};
  for (const row of ['FT8', 'FT4', 'DIG']) {
    totals[row] = {};
    DXCC_BANDS.forEach((b, i) => {
      const n = (i * 7 + row.length * 3) % 5 === 0 ? 0 : 12 + ((i * 11 + row.length) % 80);
      totals[row]![b] = { countries: n, unconfirmed: n > 40 ? 3 : 0, state: n > 40 ? 'unconfirmed' : 'worked' };
    });
  }
  return { bands: [...DXCC_BANDS], rows: ['FT8', 'FT4', 'DIG'], worked: 214, total: 340, qsos: 1830, row_worked: { FT8: 190, FT4: 90, DIG: 20 }, totals, subject: null };
}

/** Le caselle di una stazione d'esempio: FT8 e FT4 diversi, per vedere i colori per modo. */
export function sampleStationCells(): Record<string, Record<string, 'worked' | 'unconfirmed'>> {
  return {
    FT8: { '40m': 'worked', '30m': 'unconfirmed', '20m': 'worked', '17m': 'worked', '15m': 'unconfirmed' },
    FT4: { '20m': 'unconfirmed', '17m': 'worked', '10m': 'unconfirmed' },
    DIG: { '20m': 'worked', '15m': 'unconfirmed' },
  };
}

/** Etichette d'esempio: alcune accese, altre spente. */
export function sampleLabels(): DxccLabels {
  return { new_one: false, new_band: true, new_mode: true, new_grid: false, grid_band: true, grid_mode: false };
}

/** Scheda senza dati (core non raggiungibile o log vuoto): tutte le caselle restano grigie, la struttura c'e' sempre. */
export function emptyMatrix(): DxccMatrix {
  return { bands: [...DXCC_BANDS], rows: ['FT8', 'FT4', 'DIG'], worked: 0, total: 0, qsos: 0, row_worked: {}, totals: {}, subject: null };
}
