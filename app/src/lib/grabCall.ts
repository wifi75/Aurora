import { isCallsign } from '../components/digi/callLookup';

/** Un rapporto o un numero scritto con le lettere dei contest (599, 5NN, 59913, ENN): non e' un nominativo. */
const NOT_A_CALL = /^(?:[1-5][1-9N][1-9N]\w*|[0-9TNAE]+)$/;

/**
 * `<GRABCALL>`: l'ultimo nominativo comparso nel testo ricevuto (gli ultimi 300 caratteri), escluso il proprio. Dopo un «DE» vale quello
 * subito dopo (chi trasmette). Vuoto se non ce n'e'.
 */
export function grabCall(rxText: string, myCall: string): string {
  const mine = myCall.trim().toUpperCase();
  const words = rxText.slice(-300).toUpperCase().split(/[^A-Z0-9/]+/).filter((w) => w !== '');
  const ok = (w: string) => isCallsign(w) && w !== mine && !NOT_A_CALL.test(w);
  for (let i = words.length - 2; i >= 0; i -= 1) {
    const next = words[i + 1] ?? '';
    if (words[i] === 'DE' && ok(next)) return next;
  }
  for (let i = words.length - 1; i >= 0; i -= 1) {
    const w = words[i] ?? '';
    if (ok(w)) return w;
  }
  return '';
}
