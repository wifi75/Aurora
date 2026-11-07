import { useEffect, useRef, useState } from 'react';
import type { RadioMeters } from '../api/schemas';

/** Discesa dello S-meter: 12 dB al secondo (due punti S), come la lancetta di una radio che "tiene" il picco. */
export const S_RELEASE_DB_PER_S = 12;
/** Discesa della potenza: meta' del fondo letto al secondo. */
export const PO_RELEASE_PER_S = 0.5;

/** Livello S in dB rispetto a S9 (S9 = 0, 6 dB per punto S, oltre S9 i dB in piu'); `null` senza S-meter. */
export function sDb(m: RadioMeters | null | undefined): number | null {
  if (!m || (m.s_points === undefined && m.s_over_db === undefined)) return null;
  const points = m.s_points ?? 9;
  return points >= 9 ? (m.s_over_db ?? 0) : (points - 9) * 6;
}

function fromDb(db: number): Pick<RadioMeters, 's_points' | 's_over_db'> {
  return db >= 0 ? { s_points: 9, s_over_db: db } : { s_points: Math.max(0, 9 + db / 6), s_over_db: 0 };
}

/** Un passo della lancetta: sale subito al valore nuovo, scende al massimo di `release` per secondo. */
export function settle(shown: number | null, target: number | null, dtS: number, release: number): number | null {
  if (target === null) return null;
  if (shown === null || target >= shown) return target;
  return Math.max(target, shown - release * dtS);
}

/**
 * Gli strumenti con l'inerzia di quelli della radio: lo stato arriva 4 volte al secondo con il valore istantaneo, e con i segnali FT8
 * (che vanno e vengono) la lancetta saltava e cadeva tra un picco e l'altro. Salita immediata, discesa lenta: ora segue la radio.
 */
export function useMeterBallistics(meters: RadioMeters | null): RadioMeters | null {
  const target = useRef(meters);
  target.current = meters;
  const shown = useRef<{ s: number | null; po: number | null }>({ s: sDb(meters), po: meters?.power_w ?? null });
  const [, redraw] = useState(0);

  useEffect(() => {
    let frame = 0;
    let last = performance.now();
    let drawn = 0;
    let dirty = false;
    const tick = (now: number) => {
      const dt = Math.min(0.2, (now - last) / 1000);
      last = now;
      const m = target.current;
      const prev = shown.current;
      const s = settle(prev.s, sDb(m), dt, S_RELEASE_DB_PER_S);
      const po = settle(prev.po, m?.power_w ?? null, dt, Math.max(1, prev.po ?? 0) * PO_RELEASE_PER_S);
      // il valore della lancetta segue ogni fotogramma, ma il display si ridisegna al piu' 25 volte al secondo (prima 60: tutto il
      // display della radio rifatto a ogni fotogramma) e mai a finestra nascosta
      if (s !== prev.s || po !== prev.po) {
        shown.current = { s, po };
        dirty = true;
      }
      if (dirty && now - drawn >= 40 && !document.hidden) {
        drawn = now;
        dirty = false;
        redraw((n) => n + 1);
      }
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, []);

  if (!meters) return null;
  const { s, po } = shown.current;
  return {
    ...meters,
    ...(s !== null ? fromDb(s) : {}),
    ...(po !== null && meters.power_w !== undefined ? { power_w: po } : {}),
  };
}
