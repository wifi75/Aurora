/**
 * JTDX, in Impostazioni > Segnalazioni, accetta i Reply UDP (il doppio clic di Aurora) solo per certi messaggi:
 * 0 = solo CQ, 1 = CQ e 73, 2 = qualsiasi messaggio. Un Reply per un messaggio non ammesso viene ignorato in silenzio.
 */
export function replyAccepted(mode: string | undefined, text: string): boolean {
  if (mode === undefined || mode === '2') return true;
  const tokens = text.trim().toUpperCase().split(/\s+/);
  const cq = tokens[0] === 'CQ' || tokens[0] === 'QRZ';
  if (mode === '0') return cq;
  const last = tokens[tokens.length - 1];
  return cq || last === '73' || last === 'RR73';
}
