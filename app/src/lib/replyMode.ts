/**
 * JTDX, in Impostazioni > Segnalazioni, accetta i Reply UDP (il doppio clic di Aurora) solo per certi messaggi:
 * 1 = solo CQ, 2 = CQ e 73, 3 = qualsiasi messaggio (dal reverse di JTDX 159, `docs/udp-flussi.md`).
 * Un Reply per un messaggio non ammesso viene ignorato in silenzio. Un valore sconosciuto si tratta come "solo CQ".
 */
export function replyAccepted(mode: string | undefined, text: string): boolean {
  if (mode === undefined || mode === '3') return true;
  const tokens = text.trim().toUpperCase().split(/\s+/);
  const cq = tokens[0] === 'CQ' || tokens[0] === 'QRZ';
  if (mode === '2') {
    const last = tokens[tokens.length - 1];
    return cq || last === '73' || last === 'RR73';
  }
  return cq;
}
