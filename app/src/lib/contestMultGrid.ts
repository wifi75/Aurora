import { sortBands } from './contestBandColors';

/** Il minimo del punteggio che serve alla scheda di un tipo di moltiplicatore. */
export interface MultScore {
  griglia: readonly { tipo: string; valore: string; bande: readonly string[] }[];
  mancanti: readonly { tipo: string; per_banda: boolean; totali: number; valori: readonly string[] }[];
  recenti: readonly { nuovi: readonly string[] }[];
}

export interface MultCell {
  value: string;
  /** Lavorato (su almeno una banda, o una volta sola se il tipo non conta per banda). */
  worked: boolean;
  /** Le bande dove e' stato lavorato (vuoto se il tipo non conta per banda). */
  bands: string[];
  /** Portato dall'ultimo QSO che ha dato moltiplicatori nuovi. */
  fresh: boolean;
  /** E' il moltiplicatore del nominativo che si sta scrivendo. */
  pending: boolean;
}

export interface MultGridData {
  cells: MultCell[];
  worked: number;
  /** I valori possibili, se il core da' l'elenco completo (altrimenti null: la griglia ha solo i lavorati). */
  total: number | null;
  perBand: boolean;
}

const byValue = (a: string, b: string) => a.localeCompare(b, undefined, { numeric: true });

/**
 * La griglia di caselle di un tipo di moltiplicatore: una casella per ogni valore, i lavorati con le loro bande e quelli che mancano (se il core
 * ne da' l'elenco) in tono spento. `pending` = chiavi `tipo:valore` che il nominativo scritto porterebbe di nuovo.
 */
export function multGridOf(s: MultScore, tipo: string, pending: ReadonlySet<string> = new Set()): MultGridData {
  const rows = s.griglia.filter((g) => g.tipo === tipo);
  const miss = s.mancanti.find((m) => m.tipo === tipo);
  const fresh = new Set(s.recenti[0]?.nuovi ?? []);
  const key = (v: string) => `${tipo}:${v}`;
  // anche il valore che il nominativo scritto porterebbe, se l'elenco non e' completo, compare come casella «da lavorare»
  const incoming = [...pending].filter((k) => k.startsWith(`${tipo}:`)).map((k) => k.slice(tipo.length + 1));
  const values = Array.from(new Set([...rows.map((r) => r.valore), ...(miss?.valori ?? []), ...incoming])).sort(byValue);
  const cells = values.map((value) => {
    const r = rows.find((x) => x.valore === value);
    return { value, worked: r !== undefined, bands: r ? sortBands(r.bande) : [], fresh: r !== undefined && fresh.has(key(value)), pending: pending.has(key(value)) };
  });
  return { cells, worked: rows.length, total: miss ? miss.totali : null, perBand: rows.some((r) => r.bande.length > 0) || miss?.per_banda === true };
}
