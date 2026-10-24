import type { DecodedMessage } from '../api/schemas';
import { callOf } from './filters';

/** Il paese di una stazione, dall'ultimo messaggio decodificato di quella stazione (i piu' recenti per primi). */
export function countryOfCall(decodes: readonly DecodedMessage[], call: string | null | undefined): string | null {
  const wanted = call?.trim().toUpperCase();
  if (!wanted) return null;
  const found = decodes.find((d) => d.country && callOf(d.text) === wanted);
  return found?.country ?? null;
}
