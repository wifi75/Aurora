import { useSyncExternalStore } from 'react';
import type { Issue } from './diagnosis';

/** Gli avvisi della stazione calcolati da `DiagnosisBar` (audio, CAT, orologio...), condivisi con la spia di salute. */
let current: readonly Issue[] = [];
const listeners = new Set<() => void>();

export function publishHealthIssues(issues: readonly Issue[]) {
  current = issues;
  for (const l of listeners) l();
}

export function useHealthIssues(): readonly Issue[] {
  return useSyncExternalStore(
    (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    () => current,
  );
}
