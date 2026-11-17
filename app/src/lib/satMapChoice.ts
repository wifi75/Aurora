/*
 * La scelta della mappa dei satelliti: nomi riservati (gestiti solo dall'interfaccia: il core li salva come un nome qualsiasi,
 * senza barre ne' «..»), mappa effettiva, foto della Terra incluse e livelli di risoluzione. Solo logica, nessun disegno.
 */
import { SAT_BUILTIN_MAP } from './satGeo';

/** La Terra dalla NASA (foto di giorno e luci delle citta' di notte). Nome riservato nell'impostazione `map`. */
export const SAT_TERRA_MAP = ':terra';
/** La mappa semplice, vettoriale (e' il vecchio `:aurora`: chi l'aveva scelta la tiene). */
export const SAT_SIMPLE_MAP = SAT_BUILTIN_MAP;

/** Il file che si sceglie da solo, se c'e', quando l'utente non ha scelto nessuna mappa. */
export const PREFERRED_MAP = 'Contour HiRes';

export type MapChoice = { kind: 'terra' } | { kind: 'simple' } | { kind: 'file'; name: string };

/**
 * La mappa effettiva. Scelta esplicita: `:terra`, `:aurora` (semplice) o un file che c'e' ancora. Nessuna scelta (o un file sparito):
 * il file che comincia con `preferred` se c'e' (comportamento di sempre), altrimenti la Terra.
 */
export function resolveMap(setting: string, maps: readonly string[], preferred: string): MapChoice {
  if (setting === SAT_TERRA_MAP) return { kind: 'terra' };
  if (setting === SAT_SIMPLE_MAP) return { kind: 'simple' };
  if (setting && maps.includes(setting)) return { kind: 'file', name: setting };
  const pref = preferred ? maps.find((m) => m.startsWith(preferred)) : undefined;
  return pref ? { kind: 'file', name: pref } : { kind: 'terra' };
}

/** Il valore della tendina delle impostazioni per l'impostazione salvata: vuoto («Automatica») se non e' una scelta valida. */
export function selectValue(setting: string, maps: readonly string[]): string {
  return setting === SAT_TERRA_MAP || setting === SAT_SIMPLE_MAP || maps.includes(setting) ? setting : '';
}

const base = (): string => import.meta.env.BASE_URL ?? '/';

/** Gli indirizzi delle due foto incluse (cartella `public/maps`, servita con l'interfaccia: vale per il browser, il telefono e il programma). */
export function terraUrls(): { day: string; night: string } {
  return { day: `${base()}maps/terra-giorno.jpg`, night: `${base()}maps/terra-notte.jpg` };
}

/** Larghezze delle foto ridotte tenute in memoria (la foto intera e' 8192 px). */
export const PHOTO_LEVELS: readonly number[] = [1024, 2048, 4096];
export const PHOTO_FULL = 8192;
/** La foto della notte non si usa oltre questa larghezza (le luci sono sfocate di natura e la maschera costa). */
export const NIGHT_MAX_LEVEL = 4096;

/** La larghezza della foto da disegnare per avere almeno `needed` pixel sul mondo intero: la piu' piccola che basta, o la foto intera. */
export function photoLevel(needed: number, levels: readonly number[] = PHOTO_LEVELS, full = PHOTO_FULL): number {
  for (const l of levels) if (l >= needed) return l;
  return full;
}

const BORDERS_KEY = 'aurora.sat.borders';

/** «Confini» sopra la foto: spento di base; e' una preferenza di questo browser. */
export function loadBorders(): boolean {
  try {
    return window.localStorage.getItem(BORDERS_KEY) === '1';
  } catch {
    return false;
  }
}

export function saveBorders(on: boolean): void {
  try {
    window.localStorage.setItem(BORDERS_KEY, on ? '1' : '0');
  } catch {
    /* memoria del browser non disponibile */
  }
}
