import { useEffect, useRef } from 'react';

/**
 * Salva da sola una scheda di impostazioni: quando ci sono modifiche valide, dopo una breve pausa;
 * e se si esce dalla scheda con una modifica ancora in attesa, la manda subito, cosi' non si perde nulla.
 * `value` e' l'impostazione pronta da salvare (`null` se i valori non vanno), `pending` dice se ci sono modifiche.
 */
export function useAutosave<T>(value: T | null, pending: boolean, save: (v: T) => void, flush: (v: T) => void, delayMs = 600): void {
  const latest = useRef({ value, pending, flush });
  latest.current = { value, pending, flush };
  // si salva una volta per ogni valore diverso: un errore non fa ripartire il tentativo all'infinito
  const key = pending && value !== null ? JSON.stringify(value) : '';

  useEffect(() => {
    if (key === '' || value === null) return;
    const timer = setTimeout(() => save(value), delayMs);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  useEffect(
    () => () => {
      const { value: v, pending: p, flush: f } = latest.current;
      if (p && v !== null) f(v);
    },
    [],
  );
}
