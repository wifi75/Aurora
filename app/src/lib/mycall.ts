/** Il messaggio e' rivolto a me? In FT8 il primo nominativo e' il destinatario: se e' il mio, qualcuno mi sta chiamando. */
export function isCallToMe(text: string, myCall: string | null | undefined): boolean {
  const me = myCall?.trim().toUpperCase();
  if (!me) return false;
  return text.trim().toUpperCase().split(/\s+/)[0] === me;
}
