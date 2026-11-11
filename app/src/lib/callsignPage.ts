/**
 * La scheda di un nominativo su QRZ.com, da aprire nel browser (nel programma per Windows i link `target="_blank"` vanno nel browser di
 * sistema, `desktop/src-tauri/src/links.rs`). Il nominativo si codifica: «EA8/IU3ABC» resta un solo pezzo dell'indirizzo.
 */

/** Il nominativo pulito («<K1ABC>» del messaggio diventa «K1ABC»); `null` se vuoto o con caratteri che un nominativo non ha. */
export function cleanCall(call: string): string | null {
  const c = call.trim().replace(/^<|>$/g, '').toUpperCase();
  return /^[A-Z0-9/]{3,20}$/.test(c) && /\d/.test(c) ? c : null;
}

/** `https://www.qrz.com/db/CALL`; `null` se il nominativo non va. */
export function qrzUrl(call: string): string | null {
  const c = cleanCall(call);
  return c === null ? null : `https://www.qrz.com/db/${encodeURIComponent(c)}`;
}
