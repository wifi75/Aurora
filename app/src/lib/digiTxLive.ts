import { useEffect, useRef, useState } from 'react';
import { digiTxType, digiTxUnsend } from '../api/client';
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

/** Cosa fare di una modifica della finestra mentre si trasmette. */
export interface TxEdit {
  /** Il testo da mostrare (quello vecchio se la modifica tocca caratteri gia' partiti). */
  box: string;
  /** Quanti caratteri in coda togliere dalla fine (Backspace sul testo non ancora partito). */
  unsend: number;
  /** Il testo da accodare dopo. */
  append: string;
  /** I caratteri che restano in coda prima di `append`, se il motore li toglie tutti. */
  keep: number;
}

/** Il primo indice in cui due testi differiscono. */
function commonPrefix(a: string, b: string): number {
  const n = Math.min(a.length, b.length);
  let i = 0;
  while (i < n && a[i] === b[i]) i++;
  return i;
}

/**
 * Una modifica della finestra di trasmissione mentre si trasmette: si puo' scrivere in fondo (parte subito) e cancellare quello che non
 * e' ancora partito; i caratteri gia' partiti non si toccano.
 */
export function planEdit(old: string, next: string, p: TxProgress): TxEdit {
  const at = commonPrefix(old, next);
  if (at < p.sent) return { box: old, unsend: 0, append: '', keep: p.queued };
  if (at >= p.queued) return { box: next, unsend: 0, append: next.slice(p.queued), keep: p.queued };
  return { box: next, unsend: p.queued - at, append: next.slice(at), keep: at };
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
export function useDigiTxLive(box: string, setBox: (next: string | ((prev: string) => string)) => void) {
  const [progress, setProgress] = useState<TxProgress>(TX_NONE);
  const live = useRef({ box, progress });
  live.current = { box, progress };

  useEffect(
    () =>
      onDigi((ev) => {
        if (ev.kind === 'tx_sent' && ev.text) {
          const n = charCount(ev.text);
          setProgress((p) => ({ queued: p.queued, sent: Math.min(p.queued, p.sent + n) }));
        } else if (ev.kind === 'tx' && ev.state && ev.state !== 'sending') {
          const p = live.current.progress;
          if (p.queued > 0 || p.sent > 0) {
            setBox((b) => afterTransmission(b, p, ev.state === 'done'));
            setProgress(TX_NONE);
          }
        }
      }),
    [setBox],
  );

  /** Il testo gia' nella finestra e' passato al motore (inizio di una trasmissione o macro accodata). */
  const queue = (count: number) => setProgress((p) => ({ queued: p.queued + count, sent: p.sent }));

  /** Una modifica della finestra mentre si trasmette. */
  const edit = (next: string) => {
    const { box: old, progress: p } = live.current;
    const plan = planEdit(old, next, p);
    setBox(plan.box);
    if (plan.box === old) return;
    const sendRest = (queued: number, text: string) => {
      setProgress((q) => ({ queued: queued + text.length, sent: q.sent }));
      if (text !== '') void digiTxType(text).catch(() => undefined);
    };
    if (plan.unsend === 0) {
      sendRest(p.queued, plan.append);
      return;
    }
    // si toglie dal motore quello che non e' ancora partito; se qualche carattere stava gia' partendo resta nella finestra
    setProgress((q) => ({ queued: plan.keep, sent: q.sent }));
    void digiTxUnsend(plan.unsend).then(
      (removed) => {
        const kept = plan.unsend - removed;
        if (kept > 0) setBox(old.slice(0, plan.keep + kept) + plan.append);
        sendRest(plan.keep + kept, plan.append);
      },
      () => {
        setBox(old);
        setProgress((q) => ({ queued: p.queued, sent: q.sent }));
      },
    );
  };

  return { progress, queue, edit, reset: () => setProgress(TX_NONE) };
}
