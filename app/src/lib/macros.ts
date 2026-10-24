import { create } from 'zustand';
import { browserStorage } from './theme';

const KEY = 'aurora-macros';
const MAX_MACRO = 30;
const MAX_FREE_TEXT = 13;

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

/** Le cinque macro di JTDX: @ = nominativo DX, & = mio nominativo, # = rapporto. */
export function defaultMacros(): string[] {
  return ['@ TNX 73', 'TU ^ 73', '@ &', '@ #', '@ R#'];
}

export interface MacroContext {
  dx: string;
  me: string;
  report: string;
}

/** Il testo libero che risulta da una macro: maiuscolo, al massimo 13 caratteri; cio' che manca si omette. */
export function expandMacro(macro: string, ctx: MacroContext): string {
  const text = macro
    .replaceAll('@', ctx.dx)
    .replaceAll('&', ctx.me)
    .replaceAll('#', ctx.report)
    .replace(/\s+/g, ' ')
    .trim()
    .toUpperCase();
  return text.slice(0, MAX_FREE_TEXT).trim();
}

export function addMacro(macros: string[], text: string): string[] {
  const clean = text.trim();
  if (clean === '' || clean.length > MAX_MACRO || macros.includes(clean)) return macros;
  return [...macros, clean];
}

export function removeMacro(macros: string[], text: string): string[] {
  return macros.filter((m) => m !== text);
}

export function readMacros(storage: ReadableStorage): string[] {
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return defaultMacros();
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return defaultMacros();
    return parsed.filter((m): m is string => typeof m === 'string' && m.trim() !== '' && m.length <= MAX_MACRO);
  } catch {
    return defaultMacros();
  }
}

export function saveMacros(storage: WritableStorage, macros: string[]): void {
  try {
    storage?.setItem(KEY, JSON.stringify(macros));
  } catch {
    // senza archivio le macro non vengono ricordate, ma l'interfaccia funziona lo stesso
  }
}

interface MacrosState {
  macros: string[];
  add(text: string): void;
  remove(text: string): void;
  reset(): void;
}

export const useMacros = create<MacrosState>((set, get) => ({
  macros: readMacros(browserStorage()),
  add(text) {
    const next = addMacro(get().macros, text);
    saveMacros(browserStorage(), next);
    set({ macros: next });
  },
  remove(text) {
    const next = removeMacro(get().macros, text);
    saveMacros(browserStorage(), next);
    set({ macros: next });
  },
  reset() {
    saveMacros(browserStorage(), defaultMacros());
    set({ macros: defaultMacros() });
  },
}));
