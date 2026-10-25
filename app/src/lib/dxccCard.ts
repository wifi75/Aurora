import { bandOf } from './format';

/** Come si presenta una casella della scheda: verde (lavorato), arancio (lavorato ma non confermato o solo in parte); senza QSO e' grigia. */
export type CellState = 'worked' | 'unconfirmed';

/** Le sei etichette di una stazione (`GET /dxcc-matrix?call=...`): ognuna e' un si/no indipendente. */
export interface DxccLabels {
  new_one: boolean;
  new_band: boolean;
  new_mode: boolean;
  new_grid: boolean;
  grid_band: boolean;
  grid_mode: boolean;
}

export type LabelId = keyof DxccLabels;

/** Le etichette nell'ordine della scheda: il paese sopra, il locatore sotto. */
export const LABEL_IDS: readonly LabelId[] = ['new_one', 'new_band', 'new_mode', 'new_grid', 'grid_band', 'grid_mode'];

/** Le etichette del paese; le altre sono del locatore. */
export const COUNTRY_LABELS: readonly LabelId[] = ['new_one', 'new_band', 'new_mode'];

/** Il testo mostrato (sigle internazionali, uguali in tutte le lingue). */
export const LABEL_TEXT: Record<LabelId, string> = {
  new_one: 'NEW ONE',
  new_band: 'NEW BAND',
  new_mode: 'NEW MODE',
  new_grid: 'NEW GRID',
  grid_band: 'GRID BAND',
  grid_mode: 'GRID MODE',
};

export interface DxccTotalCell {
  countries: number;
  unconfirmed: number;
  state: CellState;
}

export interface DxccCountry {
  id: number;
  name: string;
  prefix: string;
  continent: string;
  /** Per riga (`FT8`, `FT4`, `DIG`, `ALL`) e colonna (`20m`, `U`...): solo le caselle con QSO. */
  cells: Record<string, Record<string, CellState>>;
}

export interface DxccSubject {
  call: string;
  country: string | null;
  prefix: string | null;
  continent: string | null;
  grid: string | null;
  band: string | null;
  mode: string | null;
  labels: DxccLabels;
  cells: Record<string, Record<string, CellState>>;
}

export interface DxccMatrix {
  bands: string[];
  rows: string[];
  worked: number;
  total: number;
  qsos: number;
  row_worked: Record<string, number>;
  totals: Record<string, Record<string, DxccTotalCell>>;
  /** Assente con `compact`. */
  countries?: DxccCountry[];
  subject: DxccSubject | null;
}

/** La stazione di cui si vogliono le etichette; tutto facoltativo. */
export interface DxccQuery {
  call?: string;
  grid?: string;
  band?: string;
  mode?: string;
  /** Senza l'elenco dei paesi: per chi vuole solo le etichette. */
  compact?: boolean;
}

/** La parte di indirizzo con i parametri (vuoti esclusi). */
export function dxccQueryString(q: DxccQuery): string {
  const p = new URLSearchParams();
  for (const key of ['call', 'grid', 'band', 'mode'] as const) {
    const v = q[key]?.trim();
    if (v) p.set(key, v);
  }
  if (q.compact) p.set('compact', '1');
  const s = p.toString();
  return s ? `?${s}` : '';
}

/** Il modo per il core: FT8 e FT4 come sono, JT65/JT9 come «JT» sono modi digitali (vanno a DIG). */
export function subjectMode(mode: string | null | undefined): string | undefined {
  const m = (mode ?? '').trim().toUpperCase();
  return m === '' ? undefined : m;
}

/** La banda (`20m`) di una frequenza di dial; `undefined` fuori dalle bande note. */
export function subjectBand(dialHz: number | null | undefined): string | undefined {
  return dialHz && dialHz > 0 ? (bandOf(dialHz) ?? undefined) : undefined;
}

/** Nome del colore di una casella: `worked`, `unconfirmed` o `none`. */
export function cellKind(state: CellState | undefined): CellState | 'none' {
  return state ?? 'none';
}

/** Stato di una casella dei totali; con zero paesi e' grigia. */
export function totalKind(cell: DxccTotalCell | undefined): CellState | 'none' {
  return cell && cell.countries > 0 ? cell.state : 'none';
}

/** Intestazione di colonna: `20m` -> `20`, `U` resta `U`. */
export function bandHeader(band: string): string {
  return band.endsWith('m') ? band.slice(0, -1) : band;
}

/** I paesi lavorati da mostrare nella tabella, filtrati per testo (nome o prefisso) e, se indicata, per riga. */
export function filterCountries(countries: readonly DxccCountry[], text: string, row: string): DxccCountry[] {
  const q = text.trim().toLowerCase();
  return countries.filter((c) => {
    if (row !== 'ALL' && Object.keys(c.cells[row] ?? {}).length === 0) return false;
    return q === '' || c.name.toLowerCase().includes(q) || c.prefix.toLowerCase().startsWith(q);
  });
}
