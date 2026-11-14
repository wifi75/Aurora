import type { LogQso } from '../api/client';
import { countryKey } from './flags';

/**
 * Cosa dice il log di una stazione, dal piu' importante:
 * - `dupe`: in gara, doppio per le regole del contest (non vale punti);
 * - `worked`: fuori gara, gia' collegata su questa banda e questo modo;
 * - `mult`: darebbe un moltiplicatore nuovo (in gara) o un paese mai collegato (fuori gara);
 * - `new_call`: nominativo mai collegato;
 * - `new_band`: mai collegato su questa banda (il modo si');
 * - `new_mode`: mai collegato in questo modo (la banda si');
 * - `new_band_mode`: la coppia banda e modo e' nuova;
 * - `null`: niente da segnalare (gia' collegato e nulla di nuovo, o log non letto).
 */
export type LogStatus = 'dupe' | 'worked' | 'mult' | 'new_call' | 'new_band_mode' | 'new_band' | 'new_mode' | null;

export interface LogIndex {
  /** Nominativo -> insieme di «banda|modo» collegati. */
  calls: Map<string, Set<string>>;
  /** Paesi collegati almeno una volta (chiave ridotta di `countryKey`). */
  countries: Set<string>;
}

const slot = (band: string, mode: string) => `${band.toLowerCase()}|${mode.toUpperCase()}`;

export function buildLogIndex(qsos: readonly Pick<LogQso, 'call' | 'band' | 'mode' | 'country'>[]): LogIndex {
  const calls = new Map<string, Set<string>>();
  const countries = new Set<string>();
  for (const q of qsos) {
    const call = q.call.trim().toUpperCase();
    if (call) {
      const set = calls.get(call) ?? new Set<string>();
      set.add(slot(q.band, q.mode));
      calls.set(call, set);
    }
    if (q.country) countries.add(countryKey(q.country));
  }
  return { calls, countries };
}

/** Banda e modo rispetto ai QSO gia' fatti con quel nominativo. */
function novelty(done: Set<string>, band: string | null, mode: string): 'new_band_mode' | 'new_band' | 'new_mode' {
  const bands = new Set<string>();
  const modes = new Set<string>();
  for (const s of done) {
    const [b, m] = s.split('|');
    bands.add(b ?? '');
    modes.add(m ?? '');
  }
  const newBand = !!band && !bands.has(band.toLowerCase());
  const newMode = !modes.has(mode.toUpperCase());
  if (newBand && !newMode) return 'new_band';
  if (newMode && !newBand) return 'new_mode';
  return 'new_band_mode';
}

/** Fuori gara: dal log locale (gia' ristretto alla finestra di `<IGNOREBEFORE>` e `<PERIOD>`). */
export function logStatus(index: LogIndex | null, call: string, country: string | null, band: string | null, mode: string): LogStatus {
  if (!index) return null;
  const done = index.calls.get(call.trim().toUpperCase());
  if (done && band && done.has(slot(band, mode))) return 'worked';
  if (country && !index.countries.has(countryKey(country))) return 'mult';
  if (!done) return 'new_call';
  return novelty(done, band, mode);
}

/** Il valore di una stazione per il contest attivo (dalla valutazione del log del contest). */
export interface ContestMark {
  call: string;
  doppio: boolean;
  molt_nuovi: readonly string[];
}

/**
 * In gara conta solo il log del contest (i QSO dentro la finestra della gara: quelli di prima non fanno mai un doppio). Il doppio segue
 * le regole del contest (stesso nominativo, banda e modo, oppure ignorando la banda o il modo come dice il regolamento); poi il
 * moltiplicatore nuovo, poi le novita' di nominativo, banda e modo rispetto al log del contest. Lo stesso slot che il regolamento non
 * considera doppio non ha segno.
 */
export function contestStatus(marks: ReadonlyMap<string, ContestMark> | null, index: LogIndex | null, call: string, band: string | null, mode: string): LogStatus {
  const key = call.trim().toUpperCase();
  const v = marks?.get(key);
  if (v?.doppio) return 'dupe';
  if (v && v.molt_nuovi.length > 0) return 'mult';
  if (!index) return null;
  const done = index.calls.get(key);
  if (!done) return 'new_call';
  if (band && done.has(slot(band, mode))) return null;
  return novelty(done, band, mode);
}
