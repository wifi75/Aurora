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

/** OM o YL dal nome: i nomi femminili piu' comuni (finiscono in A, salvo i maschili noti). */
const MALE_A = new Set(['ANDREA', 'LUCA', 'NICOLA', 'MATTIA', 'ELIA', 'ENEA', 'JOSHUA', 'SASHA', 'MISHA', 'KOSTA', 'ILJA', 'ILYA', 'MIKA', 'JONA', 'BORJA', 'GARCIA', 'COSTA', 'BATTISTA', 'TOSHIA']);
export function omOrYl(name: string): 'OM' | 'YL' {
  const n = name.trim().split(/\s+/)[0]?.toUpperCase() ?? '';
  return n.length > 2 && n.endsWith('A') && !MALE_A.has(n) ? 'YL' : 'OM';
}

/** Distanza (km) e direzione dal proprio locatore a quello del corrispondente; `null` se manca uno dei due. */
function path(c: MacroContext): { km: number; az: number } | null {
  const from = gridToLatLon(c.mywwloc);
  const to = gridToLatLon(c.wwloc);
  return from && to ? { km: distanceKm(from, to), az: bearingDeg(from, to) } : null;
}

/** I numeri in CW abbreviati (T = 0, O = 0, A = 1, N = 9) tornano cifre. */
export function fromCutNumbers(s: string): string {
  return /\d/.test(s) || /^[TOAN]+$/i.test(s.trim()) ? s.toUpperCase().replace(/[TO]/g, '0').replace(/A/g, '1').replace(/N/g, '9') : s;
}

/** L'istante del QSO gia' fatto con questo nominativo: dal log del contest se c'e', se no dal log (stessa banda e modo). */
export function beforeMs(c: MacroContext): number | null {
  return c.contestPrev?.beforeMs ?? c.before?.ms ?? null;
}
const hhmm = (ms: number) => {
  const d = new Date(ms);
  return `${pad(d.getUTCHours())}${pad(d.getUTCMinutes())}`;
};
const ymd = (ms: number) => new Date(ms).toISOString().slice(0, 10);
const hm = (ms: number) => new Date(ms).toISOString().slice(11, 16);

function family1(tag: string, arg: string | undefined, c: MacroContext): string | null {
  const p = c.prevQso;
  switch (tag) {
    case 'ACALL': return mainCall(c.call);
    case 'CALLPOST': return c.call.trim() !== '' && arg !== undefined ? `${c.call.toUpperCase()}${arg}` : '';
    case 'POST': {
      if (arg === undefined) return null;
      const [yes = '', no = ''] = arg.split(',');
      return c.call.trim() !== '' ? yes : no;
    }
    case 'OM': return omOrYl(c.name);
    case 'DIST': return path(c) ? String(path(c)?.km) : '?';
    case 'MILES': return path(c) ? String(Math.round((path(c)?.km ?? 0) / 1.609344)) : '?';
    case 'AZ': return path(c) ? String(path(c)?.az) : '?';
    case 'PREVNRR': return p?.nrr ?? '';
    case 'PREVCCALL': return mainCall(p?.call ?? '');
    case 'PREVNAME': return p?.name ?? '';
    case 'PREVQTH': return p?.qth ?? '';
    case 'PREVRSTR': return p?.rstr ?? '';
    case 'PREVCTIME': return p?.ms != null ? hhmm(p.ms) : '';
    case 'PREVSUF2': return (p?.call ?? '').replace(/\/.*$/, '').slice(-2);
    case 'NRRPOST': return c.nrr.trim() !== '' ? `${fromCutNumbers(c.nrr.trim())}${arg ?? ''}` : '';
    case 'QSOBEFORE': {
      const ms = beforeMs(c);
      return ms != null ? `${ymd(ms)} ${hm(ms)}` : '';
    }
    case 'TIME1':
    case 'QSOBEFORETIME1': {
      const ms = beforeMs(c);
      return ms != null ? hhmm(ms) : '';
    }
    case 'QSOBEFOREMODE': return c.before?.mode ?? '';
    case 'QSOBEFOREBAND': return c.before?.band ?? '';
    default: return null;
  }
}

function special1(tag: string, arg: string | undefined, r: MacroRun): boolean {
  const c = r.ctx;
  if (tag === 'ONQSOBEFORE') {
    // con il nominativo gia' lavorato si esegue la macro indicata
    const text = c.macros?.get((arg ?? '').trim().toUpperCase());
    if (beforeMs(c) != null && text !== undefined) r.expand(text);
    return true;
  }
  if (tag === 'LASTCALL') {
    const call = (c.prevQso?.call ?? c.contestPrev?.lastCall ?? '').toUpperCase();
    if (call !== '') {
      c.call = call;
      r.act({ type: 'setField', field: 'call', value: call });
    }
    return true;
  }
  return false;
}














/** I comandi che cambiano i dati della macro o espandono altro testo: `true` se il comando e' stato gestito. */
export function special(tag: string, arg: string | undefined, r: MacroRun): boolean {
  return (
    special1(tag, arg, r) ||
    false
  );
}

/** Il valore di un comando di testo in piu', o `null`. */
export function extraText(tag: string, arg: string | undefined, c: MacroContext): string | null {
  return (
    family1(tag, arg, c) ??
    null
  );
}

/** L'azione di un comando in piu', o `null`. */
export function extraAction(tag: string, arg: string | undefined): MacroAction | null {
  return (
    null
  );
}
