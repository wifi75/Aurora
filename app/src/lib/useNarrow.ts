import { useEffect, useState } from 'react';

/** Sotto questa larghezza della finestra le schede si impilano in una colonna. */
export const NARROW_QUERY = '(max-width: 900px)';

/** Vero se la finestra e' stretta (schede impilate, senza spostamenti). */
export function useNarrow(): boolean {
  const [narrow, setNarrow] = useState(() => typeof window.matchMedia === 'function' && window.matchMedia(NARROW_QUERY).matches);
  useEffect(() => {
    if (typeof window.matchMedia !== 'function') return;
    const mq = window.matchMedia(NARROW_QUERY);
    const on = () => setNarrow(mq.matches);
    mq.addEventListener('change', on);
    on();
    return () => mq.removeEventListener('change', on);
  }, []);
  return narrow;
}
