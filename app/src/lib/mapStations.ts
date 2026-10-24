import type { DecodedMessage } from '../api/schemas';
import { isGrid } from './maidenhead';

/** Una stazione di cui i messaggi dicono il locatore. */
export interface MapStation {
  call: string;
  grid: string;
  snr: number;
  /** E' un CQ (chiama tutti) e non la risposta a qualcuno. */
  cq: boolean;
  tag: DecodedMessage['tag'];
  country: string | null;
  /** L'id dell'ultimo messaggio, per sapere quale e' piu' recente. */
  id: number;
}

/**
 * Le stazioni con locatore nei messaggi, una per nominativo (vale l'ultimo messaggio). Un messaggio ha un locatore quando finisce con
 * quattro caratteri come «JN45»: «CQ DX K1ABC FN42» o «K1ABC G0XYZ IO91»; il mittente e' la parola prima del locatore.
 * «RR73» ha la stessa forma ma e' un saluto.
 */
export function stationsWithGrid(decodes: readonly DecodedMessage[]): MapStation[] {
  const byCall = new Map<string, MapStation>();
  // dal piu' vecchio al piu' recente: l'ultimo messaggio di una stazione vince
  for (const d of [...decodes].sort((a, b) => a.id - b.id)) {
    const tokens = d.text.trim().toUpperCase().split(/\s+/);
    if (tokens.length < 3) continue;
    const grid = tokens[tokens.length - 1]!;
    if (grid === 'RR73' || !isGrid(grid)) continue;
    const call = tokens[tokens.length - 2]!;
    if (!/[0-9]/.test(call) || call.length < 3) continue;
    byCall.set(call, { call, grid, snr: d.snr, cq: tokens[0] === 'CQ' || tokens[0] === 'QRZ', tag: d.tag, country: d.country ?? null, id: d.id });
  }
  return [...byCall.values()];
}
