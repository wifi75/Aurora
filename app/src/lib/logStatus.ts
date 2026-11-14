import type { LogQso } from '../api/client';
import { countryKey } from './flags';

/**
 * Cosa dice il log di una stazione: paese mai collegato (`new_dxcc`), gia' collegata su questa banda e modo (`worked`), collegata ma su
 * un'altra banda o un altro modo (`worked_other`), oppure niente (`null`). Gli stessi significati e colori dei tipi «paese nuovo» e
 * «nominativo gia' collegato» della pagina FT8.
 */
export type LogStatus = 'new_dxcc' | 'worked' | 'worked_other' | null;

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

export function logStatus(index: LogIndex | null, call: string, country: string | null, band: string | null, mode: string): LogStatus {
  if (!index) return null;
  const done = index.calls.get(call.trim().toUpperCase());
  if (done) return band && done.has(slot(band, mode)) ? 'worked' : 'worked_other';
  if (country && !index.countries.has(countryKey(country))) return 'new_dxcc';
  return null;
}
