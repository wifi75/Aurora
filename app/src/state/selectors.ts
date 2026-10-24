import type { DecodedMessage } from '../api/schemas';

/** Solo i messaggi a cui conviene rispondere: CQ e novita' (nuovo DXCC o nuova zona). Ordine invariato. */
export function cqDecodes(decodes: DecodedMessage[], limit = 40): DecodedMessage[] {
  return decodes
    .filter((d) => d.tag === 'cq' || d.tag === 'new_dxcc' || d.tag === 'new_zone')
    .slice(0, limit);
}
