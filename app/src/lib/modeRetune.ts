import type { QueryClient } from '@tanstack/react-query';
import { getEngineSettings, getFrequencies, postFrequency, type FrequencyEntry, type IaruRegion } from '../api/client';
import { useStation } from '../state/station';
import { bandPlanRows, type TextMode } from './bandplan';
import { bandOf } from './format';
import { browserStorage } from './theme';
import { modeBase } from './period';

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
 * - Il modo ha righe solo su onde lunghe o medie (FST4 sui 2200, 630 e 160 m) e la banda di adesso no: la radio resta dov'e' (il modo lavora su
 *   qualunque banda, non si va a 136 kHz). Il salto alla prima banda vale solo per i modi che esistono dai VHF in su.
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
  if (first && first.hz < 30_000_000) return null;
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

/** La frequenza della radio (in una banda radioamatoriale) prima di una pagina di sola ricezione fuori banda (WEFAX): ci si torna uscendone. */
let beforeReceiveOnly = 0;
const BEFORE_KEY = 'aurora.modenav.beforeReceiveOnly';

function savedBefore(): number {
  if (beforeReceiveOnly > 0) return beforeReceiveOnly;
  try {
    return Number(browserStorage()?.getItem(BEFORE_KEY) ?? '') || 0;
  } catch {
    return 0;
  }
}

function clearBefore(): void {
  beforeReceiveOnly = 0;
  try {
    browserStorage()?.removeItem(BEFORE_KEY);
  } catch {
    // senza archivio resta la sola memoria
  }
}

/** Da chiamare prima di spostare la radio su una frequenza fuori dalle bande radioamatoriali (stazioni meteo del WEFAX). */
export function rememberDialBeforeReceiveOnly(hz: number): void {
  if (hz <= 0 || !bandOf(hz)) return;
  beforeReceiveOnly = hz;
  try {
    browserStorage()?.setItem(BEFORE_KEY, String(hz));
  } catch {
    // senza archivio resta la sola memoria (si perde se la pagina si ricarica o e' in una finestra a parte)
  }
}

/** Porta la radio sulla frequenza giusta per le righe date; restituisce la frequenza scelta o `null` se non si e' mossa. */
async function retuneTo(client: QueryClient, rows: readonly FrequencyEntry[]): Promise<number | null> {
  const current = useStation.getState().status?.dial_hz ?? 0;
  // la radio e' fuori dalle bande (WEFAX): si torna alla banda di prima, sulla sua frequenza del modo (o sulla frequenza di prima se e' gia' una del modo)
  // se la memoria si e' persa (pagina ricaricata, finestra a parte) si usa l'ultima frequenza del gruppo FT8: mai la banda piu' bassa (1,810 MHz)
  const before = savedBefore() || lastDial('ft8');
  const back = !bandOf(current) && before > 0 ? before : 0;
  if (back > 0) clearBefore();
  const now = back > 0 ? back : current;
  const region = await regionOf(client);
  const hz = back > 0 ? (targetDial(rows, region, now) ?? now) : targetDial(rows, region, now);
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
  // «FST4-60», «FST4W-120»: le righe della tabella si chiamano FST4 e FST4W
  const base = modeBase(mode).toUpperCase();
  return retuneTo(client, table.filter((r) => r.mode.toUpperCase() === base));
}

/** Il piano di banda (`bandPlanRows`) da usare per ogni modo della pagina dei modi testuali; `null` per la fonia e i modi senza piano. */
export function textPlanOf(mode: string): TextMode | null {
  const plans: Record<string, TextMode> = { psk31: 'PSK31', psk63: 'PSK63', qpsk31: 'PSK31', throb: 'PSK31', rtty: 'RTTY', amtor: 'RTTY', cw: 'CW', olivia: 'OLIVIA', contestia: 'OLIVIA', mt63: 'OLIVIA' };
  return plans[mode.toLowerCase()] ?? null;
}

/** Lo stesso per i modi testuali e l'SSTV, che hanno il piano di banda IARU (`bandPlanRows`). */
export async function retuneForText(client: QueryClient, mode: TextMode): Promise<number | null> {
  return retuneTo(client, bandPlanRows(mode));
}

/** Frequenza di chiamata SSTV dei 20 m (manopola in USB): la banda principale dell'SSTV, dove si va entrando nella pagina. */
export const SSTV_MAIN_HZ = 14_230_000;

/** Entrando nella pagina SSTV da un'altra pagina la radio va sui 20 m (14,230 MHz); non si muove se e' gia' li' o se non c'e' una radio. */
export async function retuneForSstv(): Promise<number | null> {
  const now = useStation.getState().status?.dial_hz ?? 0;
  if (now <= 0 || Math.abs(now - SSTV_MAIN_HZ) < 1000) return null;
  await postFrequency(SSTV_MAIN_HZ);
  return SSTV_MAIN_HZ;
}
