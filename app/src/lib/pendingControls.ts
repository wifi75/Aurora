import { useEffect, useState } from 'react';
import type { ControlId } from '../api/client';

/**
 * Lo stato appena chiesto per un interruttore, tenuto per qualche secondo: il motore lo applica un attimo dopo la risposta, e la prima
 * rilettura poteva riportare il pulsante com'era (si accendeva solo al secondo clic).
 */
const HOLD_MS = 2500;
const pending = new Map<ControlId, { on: boolean; until: number }>();
const listeners = new Set<() => void>();

export function holdControl(id: ControlId, on: boolean): void {
  pending.set(id, { on, until: Date.now() + HOLD_MS });
  listeners.forEach((l) => l());
  window.setTimeout(() => listeners.forEach((l) => l()), HOLD_MS + 50);
}

/** Lo stato da mostrare: quello appena chiesto finche' vale, poi quello del motore. */
export function useShownControls(served: Partial<Record<ControlId, boolean>>): Partial<Record<ControlId, boolean>> {
  const [, tick] = useState(0);
  useEffect(() => {
    const l = () => tick((n) => n + 1);
    listeners.add(l);
    return () => {
      listeners.delete(l);
    };
  }, []);
  const now = Date.now();
  const out = { ...served };
  for (const [id, p] of pending) {
    if (p.until > now) out[id] = p.on;
    else pending.delete(id);
  }
  return out;
}

/** La richiesta non e' andata: si torna subito allo stato del motore. */
export function releaseControl(id: ControlId): void {
  pending.delete(id);
  listeners.forEach((l) => l());
}
