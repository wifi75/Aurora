import type { DecodedMessage } from '../api/schemas';
import { callOf } from './filters';
import { baseCall, senderOf } from './wishlist';

/** Finestra entro cui una stazione ricevuta conta come «vista»: gli ultimi 15 minuti. */
export const SEEN_WINDOW_MS = 15 * 60_000;

const norm = (call: string) => baseCall(call.trim().toUpperCase());

/** Il nominativo di chi trasmette il messaggio, se si riconosce. */
function talkerOf(text: string): string | null {
  return callOf(text) ?? senderOf(text)?.call ?? null;
}

/** La stazione compare tra i decode degli ultimi minuti? Il confronto ignora prefissi e suffissi portatile (K1ABC/P = K1ABC). */
export function isStationSeen(decodes: readonly DecodedMessage[], call: string, now: number = Date.now(), windowMs: number = SEEN_WINDOW_MS): boolean {
  const wanted = norm(call);
  if (wanted === '') return true;
  return decodes.some((d) => {
    if (now - d.time_ms > windowMs) return false;
    const talker = talkerOf(d.text);
    return talker !== null && norm(talker) === wanted;
  });
}

/** Chiede conferma solo se l'interruttore e' acceso e la stazione non e' tra i decode recenti. */
export function needsCallConfirm(enabled: boolean, decodes: readonly DecodedMessage[], call: string | null | undefined, now: number = Date.now()): boolean {
  if (!enabled || !call) return false;
  return !isStationSeen(decodes, call, now);
}

/** Il testo della finestra di conferma: nominativo e, se noto, il paese. */
export function callConfirmText(template: string, call: string, country?: string | null): string {
  const c = country?.trim();
  return template.replace('{call}', c ? `${call} (${c})` : call);
}
