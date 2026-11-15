/**
 * I comandi delle macro oltre a quelli di base (`macroLang.ts`), famiglia per famiglia: i valori di testo (`extraText`), i comandi che
 * cambiano i dati della macro mentre si legge (`special`) e quelli che la pagina esegue (`extraAction`, quasi tutti `cmd`, vedi
 * `macroRuntime.ts`).
 */
import { bearingDeg, distanceKm } from './geo';
import { parseIgnoreBefore } from './logWindow';
import { gridToLatLon } from './maidenhead';
import type { MacroAction, MacroContext } from './macroLang';

/** Il nominativo senza i suffissi (/P, /M...) e senza il prefisso di un altro paese. */
export function mainCall(call: string): string {
  const parts = call.toUpperCase().split('/').filter(Boolean);
  if (parts.length <= 1) return parts[0] ?? '';
  // si tiene la parte piu' lunga e con una cifra: quella e' il nominativo vero
  return [...parts].sort((a, b) => Number(/\d/.test(b)) - Number(/\d/.test(a)) || b.length - a.length)[0] ?? '';
}

/** Lo stato di una macro mentre si espande: i dati (si possono cambiare) e dove finiscono testo e comandi. */
export interface MacroRun {
  ctx: MacroContext;
  text: (s: string) => void;
  act: (a: MacroAction) => void;
  /** Espande un altro pezzo di script un livello piu' in basso (macro dentro una macro). */
  expand: (script: string) => void;
  setHidden: (hidden: boolean) => void;
}

const pad = (n: number, w = 2) => String(n).padStart(w, '0');
const cmd = (tag: string, arg?: string): MacroAction => ({ type: 'cmd', tag, arg: (arg ?? '').trim() });















/** I comandi che cambiano i dati della macro o espandono altro testo: `true` se il comando e' stato gestito. */
export function special(tag: string, arg: string | undefined, r: MacroRun): boolean {
  return (
    false
  );
}

/** Il valore di un comando di testo in piu', o `null`. */
export function extraText(tag: string, arg: string | undefined, c: MacroContext): string | null {
  return (
    null
  );
}

/** L'azione di un comando in piu', o `null`. */
export function extraAction(tag: string, arg: string | undefined): MacroAction | null {
  return (
    null
  );
}
