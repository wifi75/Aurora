/**
 * Il testo del tasto del periodo di trasmissione. Il motore lo cambia da solo con lo stato e col modo («TX 00/30» e «TX 15/45»
 * in FT8, «TX 00» in FT4...): si usa quello che scrive lui, riscritto «Tx ...». Se non si e' ancora letto, per FT8 si ricostruisce
 * (acceso = periodo pari); per gli altri modi null.
 */
export function periodLabel(on: boolean, mode: string | undefined, engineLabel = ''): string | null {
  const label = engineLabel.trim();
  if (label !== '') return label.replace(/^tx/i, 'Tx');
  if (mode === 'FT8') return on ? 'Tx 00/30' : 'Tx 15/45';
  return null;
}
