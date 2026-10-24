export interface QsoContext {
  myCall: string;
  myGrid: string;
  dxCall: string;
  /** Rapporto di segnale come lo scrive JTDX ("-10", "5"...). */
  report: string;
}

function signed(report: string): string {
  const n = Number.parseInt(report, 10);
  const value = Number.isFinite(n) ? n : 0;
  return `${value < 0 ? '-' : '+'}${String(Math.abs(value)).padStart(2, '0')}`;
}

/** I sei messaggi standard di JTDX (Tx1-Tx6) per il QSO in corso; vuoti se non c'e' una stazione DX. */
export function txMessages({ myCall, myGrid, dxCall, report }: QsoContext): string[] {
  const grid = myGrid.slice(0, 4).toUpperCase();
  const cq = `CQ ${myCall} ${grid}`.trim();
  if (!dxCall) return ['', '', '', '', '', cq];
  const head = `${dxCall} ${myCall}`;
  const rpt = signed(report);
  return [`${head} ${grid}`, `${head} ${rpt}`, `${head} R${rpt}`, `${head} RR73`, `${head} 73`, cq];
}

/** Indice (0-5) del messaggio che JTDX sta trasmettendo, o -1. */
export function activeTx(messages: string[], current: string | null | undefined): number {
  const wanted = current?.trim().toUpperCase();
  if (!wanted) return -1;
  return messages.findIndex((m) => m !== '' && m.toUpperCase() === wanted);
}
