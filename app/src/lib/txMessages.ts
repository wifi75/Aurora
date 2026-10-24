export interface QsoContext {
  myCall: string;
  myGrid: string;
  dxCall: string;
  /** Rapporto di segnale come lo scrive il motore ("-10", "5"...). */
  report: string;
}

function signed(report: string): string {
  const n = Number.parseInt(report, 10);
  const value = Number.isFinite(n) ? n : 0;
  return `${value < 0 ? '-' : '+'}${String(Math.abs(value)).padStart(2, '0')}`;
}

/** I sei messaggi standard del motore (Tx1-Tx6) per il QSO in corso; vuoti se non c'e' una stazione DX. */
export function txMessages({ myCall, myGrid, dxCall, report }: QsoContext): string[] {
  const grid = myGrid.slice(0, 4).toUpperCase();
  const cq = `CQ ${myCall} ${grid}`.trim();
  if (!dxCall) return ['', '', '', '', '', cq];
  const head = `${dxCall} ${myCall}`;
  const rpt = signed(report);
  return [`${head} ${grid}`, `${head} ${rpt}`, `${head} R${rpt}`, `${head} RR73`, `${head} 73`, cq];
}

/** Indice (0-5) del messaggio che il motore sta trasmettendo, o -1. */
export function activeTx(messages: string[], current: string | null | undefined): number {
  const wanted = current?.trim().toUpperCase();
  if (!wanted) return -1;
  return messages.findIndex((m) => m !== '' && m.toUpperCase() === wanted);
}

/**
 * Quale messaggio (0-5, cioe' Tx1-Tx6) tocca trasmettere ora, secondo la fase del QSO: guarda l'ultimo
 * messaggio che la stazione chiamata ha mandato a noi (elenco dal piu' recente).
 */
export function nextTx(decodes: ReadonlyArray<{ text: string }>, myCall: string, dxCall: string): number {
  const me = myCall.trim().toUpperCase();
  const dx = dxCall.trim().toUpperCase();
  if (!dx) return 5;
  for (const d of decodes) {
    const tokens = d.text.trim().toUpperCase().split(/\s+/);
    if (tokens[0] !== me || tokens[1] !== dx) continue;
    const payload = tokens.slice(2).join(' ');
    if (/^(RRR|RR73|73)$/.test(payload)) return 4;
    if (/^R[+-]\d{2}$/.test(payload)) return 3;
    if (/^[+-]\d{2}$/.test(payload)) return 2;
    if (/^[A-R]{2}\d{2}$/.test(payload)) return 1;
  }
  return 0;
}
