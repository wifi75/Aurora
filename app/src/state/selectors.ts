import type { DecodedMessage } from '../api/schemas';

const WORTH_ANSWERING = new Set(['cq', 'new_dxcc', 'new_dxcc_band', 'new_zone', 'new_grid']);

/** Solo i messaggi a cui conviene rispondere: CQ e novita' di ogni genere. Ordine invariato. */
export function cqDecodes(decodes: DecodedMessage[], limit = 40): DecodedMessage[] {
  return decodes
    .filter((d) => d.tag !== null && WORTH_ANSWERING.has(d.tag))
    .slice(0, limit);
}
