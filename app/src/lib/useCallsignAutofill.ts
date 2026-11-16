import { useEffect, useRef } from 'react';
import { lookupCallsign } from '../components/CallsignTip';
import { filledFrom, isCallsign } from '../components/digi/callLookup';

/** Attesa dopo l'ultimo tasto prima di cercare il nominativo (non si cerca a ogni lettera). */
export const AUTOFILL_DELAY_MS = 400;

/** I campi del QSO che la ricerca del nominativo puo' riempire. */
export type AutofillKey = 'name' | 'qth' | 'wwloc';
export type AutofillValues = Partial<Record<AutofillKey, string>>;

/**
 * Scrivendo un nominativo, dopo una breve attesa si cercano i dati della stazione con la ricerca gia' configurata in Aurora (QRZ.com o
 * HamQTH, `/callsign/{call}`, con la stessa memoria delle altre schermate: nessuna ricerca doppia) e si riempiono nome, QTH e (se il
 * campo c'e') locatore. Si scrive solo nei campi vuoti o ancora uguali al valore messo da qui per il nominativo di prima: quello scritto
 * a mano non si tocca. Cambiando nominativo i valori messi da qui si tolgono (o si sostituiscono con quelli del nuovo). Con la ricerca
 * spenta o senza risultato non succede niente e nessun errore.
 *
 * `values` contiene solo i campi che l'inserimento ha (chi non ha il locatore non lo passa); `apply` riceve il gruppo di campi da cambiare.
 */
export function useCallsignAutofill(call: string, values: AutofillValues, apply: (patch: AutofillValues) => void, enabled = true): void {
  const auto = useRef<AutofillValues>({});
  const last = useRef('');
  const vals = useRef(values);
  vals.current = values;
  const applyRef = useRef(apply);
  applyRef.current = apply;
  const key = call.trim().toUpperCase();

  useEffect(() => {
    const keys = Object.keys(vals.current) as AutofillKey[];
    // nominativo cambiato: quello che aveva messo la ricerca di prima, se non e' stato toccato, si toglie
    if (key !== last.current) {
      last.current = key;
      const clear: AutofillValues = {};
      for (const k of keys) {
        const mine = auto.current[k];
        if (mine !== undefined && mine !== '' && vals.current[k] === mine) clear[k] = '';
      }
      auto.current = {};
      if (Object.keys(clear).length > 0) applyRef.current(clear);
    }
    if (!enabled || !isCallsign(key)) return;
    let gone = false;
    const id = window.setTimeout(() => {
      void lookupCallsign(key)
        .then((res) => {
          if (gone || !res) return;
          const found = filledFrom(res);
          const patch: AutofillValues = {};
          for (const k of Object.keys(vals.current) as AutofillKey[]) {
            const now = vals.current[k] ?? '';
            if (now !== '' && auto.current[k] !== now) continue;
            if (found[k] === '') continue;
            auto.current[k] = found[k];
            if (found[k] !== now) patch[k] = found[k];
          }
          if (Object.keys(patch).length > 0) applyRef.current(patch);
        })
        .catch(() => undefined);
    }, AUTOFILL_DELAY_MS);
    return () => {
      gone = true;
      window.clearTimeout(id);
    };
  }, [key, enabled]);
}
