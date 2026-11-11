import type { QueryClient } from '@tanstack/react-query';
import { getEngineSettings, getFrequencies, postFrequency, type FrequencyEntry, type IaruRegion } from '../api/client';
import { useStation } from '../state/station';
import { bandPlanRows, type TextMode } from './bandplan';
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

/** Una riga vale se la Regione IARU scelta e' «tutte», se la riga vale ovunque o se e' della regione scelta (come il filtro di JTDX). */
function seenBy(chosen: IaruRegion, row: IaruRegion): boolean {
  return chosen === 'ALL' || row === 'ALL' || row === chosen;
}

/**
 * Dove deve andare la radio per un modo, date le sue righe di frequenza, la Regione IARU e la frequenza di adesso; `null` se non deve muoversi.
 * - Gia' su una frequenza del modo: resta li'.
 * - La banda di adesso ha una frequenza del modo (FT8 -> FT4, MSK144 -> FT8 sui 6 m): si resta sulla banda, sulla frequenza principale del modo.
 * - La banda di adesso non ha il modo (MSK144 sui 15 m: esiste solo da 6 m in su): si va alla prima banda dove esiste, la piu' bassa, e li' sulla
 *   frequenza principale della Regione scelta (R1 -> 6 m 50.380).
 */
export function targetDial(rows: readonly FrequencyEntry[], region: IaruRegion, nowHz: number): number | null {
  const seen = rows.filter((r) => seenBy(region, r.region));
  const use = seen.length > 0 ? seen : [...rows];
  if (use.length === 0 || nowHz <= 0 || use.some((r) => r.hz === nowHz)) return null;
  const byHz = (list: readonly FrequencyEntry[]) => [...list].sort((a, b) => a.hz - b.hz);
  const main = (list: readonly FrequencyEntry[]) => {
    const sorted = byHz(list);
    return sorted.find((r) => r.preferred) ?? sorted[0];
  };
  const band = bandOf(nowHz);
  const here = band ? use.filter((r) => r.band === band) : [];
  if (here.length > 0) return main(here)?.hz ?? null;
  const first = byHz(use)[0];
  return first ? (main(use.filter((r) => r.band === first.band))?.hz ?? null) : null;
}

async function regionOf(client: QueryClient): Promise<IaruRegion> {
  try {
    const s = await client.fetchQuery({ queryKey: ['engine-settings'], queryFn: getEngineSettings, staleTime: 30_000 });
    const v = String((s.values as Record<string, unknown>).Region ?? 'ALL');
    return v === 'R1' || v === 'R2' || v === 'R3' ? v : 'ALL';
  } catch {
    return 'ALL';
  }
}

/** Porta la radio sulla frequenza giusta per le righe date; restituisce la frequenza scelta o `null` se non si e' mossa. */
async function retuneTo(client: QueryClient, rows: readonly FrequencyEntry[]): Promise<number | null> {
  const now = useStation.getState().status?.dial_hz ?? 0;
  const hz = targetDial(rows, await regionOf(client), now);
  if (hz === null) return null;
  await postFrequency(hz);
  return hz;
}

/**
 * Dopo un cambio di modo del motore (FT8, FT4, MSK144...) la radio resta dov'era: se quella frequenza non e' del nuovo modo la porta su una che lo e'
 * (vedi `targetDial`). Il comando e' quello della frequenza di dial della radio.
 */
export async function retuneForMode(client: QueryClient, mode: string): Promise<number | null> {
  const table = await client.fetchQuery({ queryKey: ['frequencies'], queryFn: getFrequencies, staleTime: 60_000 });
  return retuneTo(client, table.filter((r) => r.mode.toUpperCase() === mode.toUpperCase()));
}

/** Lo stesso per i modi testuali e l'SSTV, che hanno il piano di banda IARU (`bandPlanRows`). */
export async function retuneForText(client: QueryClient, mode: TextMode): Promise<number | null> {
  return retuneTo(client, bandPlanRows(mode));
}
