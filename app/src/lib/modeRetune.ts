import type { QueryClient } from '@tanstack/react-query';
import { getFrequencies, postRxFrequency } from '../api/client';
import { useStation } from '../state/station';
import { bandOf } from './format';
import { browserStorage } from './theme';

const FREQ_KEY = 'aurora.modenav.freq';

/** L'ultima frequenza della radio usata in un gruppo di modi (`ft8` o `vhf`). */
export function lastDial(group: string): number {
  try {
    return Number(browserStorage()?.getItem(`${FREQ_KEY}.${group}`) ?? '') || 0;
  } catch {
    return 0;
  }
}

export function rememberDial(group: string, hz: number): void {
  try {
    browserStorage()?.setItem(`${FREQ_KEY}.${group}`, String(hz));
  } catch {
    // senza archivio si riparte dalla frequenza principale
  }
}

/**
 * Dopo un cambio di modo la radio resta dov'era (es. 40 m per FT8): se quella frequenza non e' del nuovo modo la porta su una che lo e'.
 * Prima la principale della stessa banda (FT8 -> FT4), poi l'ultima usata nel gruppo, poi la prima principale della tabella (MSK144: 6 m).
 */
export async function retuneForMode(client: QueryClient, mode: string, group: string): Promise<void> {
  const table = await client.fetchQuery({ queryKey: ['frequencies'], queryFn: getFrequencies, staleTime: 60_000 });
  const rows = table.filter((r) => r.mode.toUpperCase() === mode.toUpperCase());
  const now = useStation.getState().status?.dial_hz ?? 0;
  if (rows.length === 0 || now <= 0 || rows.some((r) => r.hz === now)) return;
  const band = bandOf(now);
  const sameBand = band ? rows.find((r) => r.band === band && r.preferred) ?? rows.find((r) => r.band === band) : undefined;
  const saved = rows.find((r) => r.hz === lastDial(group));
  const first = [...rows].sort((a, b) => a.hz - b.hz).find((r) => r.preferred) ?? rows[0];
  const pick = sameBand ?? saved ?? first;
  if (pick) await postRxFrequency(pick.hz);
}
