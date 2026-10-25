import type { DecodedMessage } from '../api/schemas';

/** Entro questo tempo da una mia trasmissione verso una stazione, un messaggio a me da lei e' una RISPOSTA alla mia chiamata. */
export const ANSWER_WINDOW_MS = 15 * 60_000;

/** I nominativi a cui ho trasmesso (il primo nominativo del messaggio Tx), con l'istante dell'ultima trasmissione. */
export function transmittedTo(txLog: readonly DecodedMessage[]): Map<string, number> {
  const out = new Map<string, number>();
  for (const m of txLog) {
    const call = m.text.trim().split(/\s+/)[0]?.toUpperCase();
    if (!call || call === 'CQ' || call === 'QRZ') continue;
    const prev = out.get(call);
    if (prev === undefined || m.time_ms > prev) out.set(call, m.time_ms);
  }
  return out;
}

/** Il messaggio a me e' di una stazione a cui avevo trasmesso da poco: quindi ha risposto alla mia chiamata (invece di chiamarmi per prima). */
export function isAnswerToMyCall(text: string, timeMs: number, sent: ReadonlyMap<string, number>): boolean {
  const caller = text.trim().split(/\s+/)[1]?.toUpperCase();
  if (!caller) return false;
  const when = sent.get(caller);
  if (when === undefined) return false;
  const dt = timeMs - when;
  return dt >= 0 && dt <= ANSWER_WINDOW_MS;
}
