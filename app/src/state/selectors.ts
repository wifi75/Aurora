import type { DecodedMessage } from '../api/schemas';

/** Tutto cio' che e' un CQ o una novita'; restano fuori i messaggi senza etichetta e le risposte automatiche. */
const WORTH_ANSWERING = new Set([
  'cq', 'new_dxcc', 'new_dxcc_band', 'new_zone', 'new_zone_band', 'new_itu', 'new_itu_band',
  'new_grid', 'new_grid_band', 'new_px', 'new_px_band', 'new_call', 'new_call_band', 'worked_call',
]);

/** Solo i messaggi a cui conviene rispondere: CQ e novita' di ogni genere. Ordine invariato. */
export function cqDecodes(decodes: DecodedMessage[], limit = 40): DecodedMessage[] {
  return decodes
    .filter((d) => d.tag !== null && WORTH_ANSWERING.has(d.tag))
    .slice(0, limit);
}
