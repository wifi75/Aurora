import { useEffect, useRef, useState } from 'react';
import { digiTxReplace } from '../api/client';
import { onDigi } from './digiBus';

/**
 * La finestra di trasmissione durante una trasmissione a flusso: i primi `queued` caratteri sono passati al motore, i primi `sent` sono
 * gia' usciti via radio (evento `tx_sent`). Il testo oltre `queued` e' quello non ancora passato.
 */
export interface TxProgress {
  queued: number;
  sent: number;
}

export const TX_NONE: TxProgress = { queued: 0, sent: 0 };

/** Cosa fare di una modifica della finestra mentre si trasmette: `null` se tocca caratteri gia' partiti (si tiene il testo di prima),
 * altrimenti sostituire la coda del motore da `from` con `text`. */
export interface TxEdit {
  from: number;
  text: string;
}

/** Il primo indice in cui due testi differiscono. */
function commonPrefix(a: string, b: string): number {
  const n = Math.min(a.length, b.length);
  let i = 0;
  while (i < n && a[i] === b[i]) i++;
  return i;
}

/**
 * Una modifica della finestra di trasmissione mentre si trasmette: in qualunque punto della parte non ancora partita si corregge,
 * cancella o aggiunge (il motore sostituisce la coda dal primo carattere cambiato); i caratteri gia' partiti non si toccano.
 */
export function planEdit(old: string, next: string, p: TxProgress): TxEdit | null {
  const at = commonPrefix(old, next);
  if (at < p.sent) return null;
  const from = Math.min(at, p.queued);
  return { from, text: next.slice(from) };
}

/** Quanti caratteri contare per un testo arrivato dal motore (per punti di codice, come li conta lui). */
export function charCount(text: string): number {
  return Array.from(text).length;
}

/** La finestra dopo la fine della trasmissione: via i caratteri partiti (tutti quelli in coda se e' finita bene). */
export function afterTransmission(box: string, p: TxProgress, done: boolean): string {
  return box.slice(done ? Math.max(p.queued, p.sent) : p.sent);
}

/**
 * Lo stato della finestra di trasmissione durante una trasmissione a flusso: segue gli eventi `tx_sent`, toglie il testo partito alla
 * fine e porta al motore quello che si scrive mentre si trasmette (`edit`).
 */
export function useDigiTxLive(box: string, setBox: (next: string | ((prev: string) => string)) => void, onSent?: (text: string) => void) {
  const [progress, setProgress] = useState<TxProgress>(TX_NONE);
  const live = useRef({ box, progress, onSent });
  live.current = { box, progress, onSent };

  useEffect(
    () =>
      onDigi((ev) => {
        if (ev.kind === 'tx_sent' && ev.text) {
          const n = charCount(ev.text);
          setProgress((p) => ({ queued: p.queued, sent: Math.min(p.queued, p.sent + n) }));
        } else if (ev.kind === 'tx' && ev.state && ev.state !== 'sending') {
          const p = live.current.progress;
          if (p.queued > 0 || p.sent > 0) {
            // il testo partito esce dalla casella e va nello storico della finestra di trasmissione
            const before = live.current.box;
            const rest = afterTransmission(before, p, ev.state === 'done');
            live.current.onSent?.(before.slice(0, before.length - rest.length));
            setBox((b) => afterTransmission(b, p, ev.state === 'done'));
            setProgress(TX_NONE);
          }
        }
      }),
    [setBox],
  );

  /** Il testo gia' nella finestra e' passato al motore (inizio di una trasmissione o macro accodata). */
  const queue = (count: number) => setProgress((p) => ({ queued: p.queued + count, sent: p.sent }));

  /** Una modifica della finestra mentre si trasmette: la coda del motore si sostituisce dal primo carattere cambiato. Se quel punto e'
   * gia' partito o sta per partire (il motore lo rifiuta) la finestra torna com'era. */
  const edit = (next: string) => {
    const { box: old, progress: p } = live.current;
    const plan = planEdit(old, next, p);
    if (plan === null) {
      setBox(old);
      return;
    }
    setBox(next);
    setProgress((q) => ({ queued: next.length, sent: q.sent }));
    const undo = () => {
      setBox(old);
      setProgress((q) => ({ queued: p.queued, sent: q.sent }));
    };
    void digiTxReplace(plan.from, plan.text).then((ok) => (ok ? undefined : undo()), undo);
  };

  return { progress, queue, edit, reset: () => setProgress(TX_NONE) };
}
