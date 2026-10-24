/**
 * Il testo del tasto del periodo di trasmissione: accesa e' il periodo pari, spenta il dispari, e il testo dice a quali
 * secondi si trasmette (FT8: 00/30 o 15/45; FT4, con slot da 7,5 s: 00/15 o 7,5/22,5). Per gli altri modi null.
 */
export function periodLabel(on: boolean, mode: string | undefined): string | null {
  if (mode === 'FT8') return on ? 'Tx 00/30' : 'Tx 15/45';
  if (mode === 'FT4') return on ? 'Tx 00/15' : 'Tx 7,5/22,5';
  return null;
}
