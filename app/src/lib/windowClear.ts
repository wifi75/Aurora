import type { DecodedMessage } from '../api/schemas';

/**
 * «Pulisci» di una sola finestra (Attivita' di banda o Finestra RX): non tocca l'elenco condiviso (lo svuota solo «Cancella»),
 * ricorda fin dove la finestra e' stata pulita. Le decodifiche hanno id crescenti, le nostre righe Tx id negativi decrescenti.
 */
export interface ClearMark {
  /** Id piu' alto tra le decodifiche presenti al momento del clic. */
  decode: number;
  /** Id piu' basso (il piu' recente) tra le righe Tx presenti al momento del clic. */
  tx: number;
}

export const NO_MARK: ClearMark = { decode: Number.NEGATIVE_INFINITY, tx: Number.POSITIVE_INFINITY };

export function markNow(decodes: readonly DecodedMessage[], txLog: readonly DecodedMessage[]): ClearMark {
  return {
    decode: decodes.reduce((m, d) => Math.max(m, d.id), NO_MARK.decode),
    tx: txLog.reduce((m, d) => Math.min(m, d.id), NO_MARK.tx),
  };
}

/** Toglie quello che era gia' presente quando la finestra e' stata pulita. */
export function afterMark(list: DecodedMessage[], mark: ClearMark): DecodedMessage[] {
  if (mark === NO_MARK) return list;
  return list.filter((d) => (d.id < 0 ? d.id < mark.tx : d.id > mark.decode));
}
