/**
 * La via QSL del calendario NG3K accorciata per la riga dell'elenco DXpedition: «LoTW», «OQRS», «via M0OXO», al massimo tre pezzi.
 * Esempi: «M0OXO OQRS; LoTW» → «via M0OXO · OQRS · LoTW»; «Club Log OQRS» → «OQRS»; «Home Call» → «Home Call». Vuota se non c'e'.
 */
export function qslShort(qsl: string): string {
  const text = qsl.trim();
  if (!text) return '';
  const up = text.toUpperCase();
  const parts: string[] = [];
  // il primo nominativo (lettere e almeno una cifra, anche con «/»), che non sia una sigla di servizio
  const call = up.match(/\b(?=[A-Z0-9/]*\d)(?=[A-Z0-9/]*[A-Z])[A-Z0-9]{1,3}\d[A-Z0-9/]*[A-Z]\b/);
  if (call) parts.push(`via ${call[0]}`);
  if (/\bOQRS\b|CLUB\s*LOG|CLUBLOG/.test(up)) parts.push('OQRS');
  if (/\bLOTW\b/.test(up)) parts.push('LoTW');
  if (parts.length === 0) {
    if (/\bH(OME)?\s*CALL\b/.test(up)) return 'Home Call';
    if (/\bBUR(EAU|O)\b/.test(up)) return 'Bureau';
    // altro testo: le prime parole, corte
    return text.length > 18 ? `${text.slice(0, 17)}…` : text;
  }
  return parts.slice(0, 3).join(' · ');
}
