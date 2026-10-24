import { create } from 'zustand';

/** I sei messaggi di trasmissione del motore (Tx1-Tx6) e quale e' selezionato (1-6), letti dalla sua finestra. */
export interface EngineTx {
  messages: string[];
  selected: number | null;
  /** Il testo della casella del periodo come lo scrive il motore ("TX 00/30", "TX 15/45", "TX 00"...). */
  minute: string;
}

/** Quanto vale la scelta fatta a mano prima che il motore la confermi: oltre, vince quello che dice il motore. */
const CHOSEN_MS = 6_000;

interface EngineTxState {
  tx: EngineTx | null;
  /** Messaggio scelto a mano con un clic e non ancora confermato dal motore. */
  chosen: number | null;
  chosenAt: number;
  /** Quale era selezionato prima della scelta, per tornarci se fallisce. */
  before: number | null;
  /** Lettura nuova dal motore. Una scelta appena fatta a mano resta finche' il motore non la riporta (o scade). */
  set(tx: EngineTx | null): void;
  /** Mostra subito il messaggio scelto a mano. */
  choose(n: number): void;
  /** La scelta non e' andata a buon fine: si torna a quello che dice il motore. */
  unchoose(): void;
}

export const useEngineTx = create<EngineTxState>((set) => ({
  tx: null,
  chosen: null,
  chosenAt: 0,
  before: null,
  set(tx) {
    set((s) => {
      if (s.chosen === null) return { tx };
      const confirmed = tx?.selected === s.chosen;
      const expired = Date.now() - s.chosenAt > CHOSEN_MS;
      if (confirmed || expired || !tx) return { tx, chosen: null };
      return { tx: { ...tx, selected: s.chosen } };
    });
  },
  choose(n) {
    set((s) => ({ chosen: n, chosenAt: Date.now(), before: s.chosen === null ? (s.tx?.selected ?? null) : s.before, tx: s.tx ? { ...s.tx, selected: n } : s.tx }));
  },
  unchoose() {
    set((s) => ({ chosen: null, tx: s.tx ? { ...s.tx, selected: s.before } : s.tx }));
  },
}));

/** Il testo del campo `tx` dell'API, se valido: sei messaggi e una selezione tra 1 e 6 (o nessuna). */
export function parseEngineTx(raw: unknown): EngineTx | null {
  if (!raw || typeof raw !== 'object') return null;
  const { messages, selected, minute } = raw as { messages?: unknown; selected?: unknown; minute?: unknown };
  if (!Array.isArray(messages) || messages.length !== 6 || !messages.every((m) => typeof m === 'string')) return null;
  const n = typeof selected === 'number' && selected >= 1 && selected <= 6 ? selected : null;
  return { messages: messages as string[], selected: n, minute: typeof minute === 'string' ? minute.trim() : '' };
}
