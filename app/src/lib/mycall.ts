/** Il messaggio e' rivolto a me? In FT8 il primo nominativo e' il destinatario: se e' il mio, qualcuno mi sta chiamando. */
export function isCallToMe(text: string, myCall: string | null | undefined): boolean {
  const me = myCall?.trim().toUpperCase();
  if (!me) return false;
  return text.trim().toUpperCase().split(/\s+/)[0] === me;
}

/** Un messaggio di chiusura del QSO (RR73 o 73): collegamento fatto, non c'e' piu' nulla da rispondere. */
export function isClosingMessage(text: string): boolean {
  const last = text.trim().toUpperCase().split(/\s+/).pop();
  return last === 'RR73' || last === '73';
}
