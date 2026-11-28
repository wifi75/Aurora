import type { DecodedMessage } from '../api/schemas';
import { parseSpecialCq, type SpecialTag } from './specialCq';
import type { SpecialCqPrefs } from './specialCqPrefs';

/** Lo stesso nominativo fa suonare l'avviso del CQ speciale al massimo una volta ogni 10 minuti. */
export const SPECIAL_COOLDOWN_MS = 10 * 60_000;

export interface SpecialPick {
  lastId: number | null;
  /** Il CQ speciale che deve suonare (il primo dei messaggi nuovi), se c'e'. */
  call?: string;
  tag?: SpecialTag;
}

/**
 * Tra i messaggi arrivati dopo `lastId` cerca un CQ speciale con l'avviso sonoro acceso per il suo tag. Alla prima chiamata (`lastId`
 * nullo) ricorda solo dove siamo: i messaggi gia' presenti non suonano. `seen` ricorda quando ogni nominativo ha suonato (senza barre:
 * IU3ABC/P e IU3ABC sono la stessa stazione); e' separato dalle altre notifiche, cosi' un attivatore gia' segnalato come «nuovo
 * nominativo» suona lo stesso al suo CQ POTA.
 */
export function pickSpecialCq(decodes: readonly DecodedMessage[], lastId: number | null, prefs: SpecialCqPrefs, seen: Map<string, number>, now: number): SpecialPick {
  const newest = decodes.reduce<number | null>((max, d) => (max === null || d.id > max ? d.id : max), null);
  if (lastId === null) return { lastId: newest };
  const next = newest === null ? lastId : Math.max(lastId, newest);
  if (!prefs.enabled) return { lastId: next };
  let found: SpecialPick = { lastId: next };
  // dal piu' vecchio al piu' recente: suona il primo; gli altri dello stesso giro restano segnati e non suonano al giro dopo
  for (const d of decodes.filter((x) => x.id > lastId && x.id >= 0).sort((a, b) => a.id - b.id)) {
    const cq = parseSpecialCq(d.text);
    if (!cq || !prefs.sound[cq.tag]) continue;
    const at = seen.get(cq.base);
    if (at !== undefined && now - at < SPECIAL_COOLDOWN_MS) continue;
    seen.set(cq.base, now);
    if (found.call === undefined) found = { lastId: next, call: cq.call, tag: cq.tag };
  }
  // la memoria non cresce senza fine: si tolgono i nominativi gia' scaduti
  if (seen.size > 2000) for (const [k, at] of seen) if (now - at >= SPECIAL_COOLDOWN_MS) seen.delete(k);
  return found;
}
