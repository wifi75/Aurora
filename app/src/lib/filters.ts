import { create } from 'zustand';
import type { DecodedMessage } from '../api/schemas';
import { browserStorage } from './theme';

export const CONTINENTS = ['AF', 'AN', 'AS', 'EU', 'NA', 'OC', 'SA'] as const;
export type OnlyMode = 'all' | 'cq' | 'cq_rrr' | 'cq_rr73';

/** Filtri dell'elenco dei messaggi, come la scheda Filtri del motore. Solo visualizzazione: nulla viene perso. */
export interface Filters {
  hideContinents: string[];
  hideCountries: string[];
  hideCalls: string[];
  only: OnlyMode;
  hideNonStandard: boolean;
  hideOwnContinent: boolean;
  /** Nasconde i CQ di stazioni gia' collegate su questa banda (il motore le segna `worked_call`); gli altri messaggi restano. */
  hideWorked: boolean;
}

const KEY = 'aurora-filters';
const ONLY: readonly string[] = ['all', 'cq', 'cq_rrr', 'cq_rr73'];

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

export function defaultFilters(): Filters {
  return { hideContinents: [], hideCountries: [], hideCalls: [], only: 'all', hideNonStandard: false, hideOwnContinent: false, hideWorked: false };
}

const isCallToken = (token: string) =>
  token.length >= 3 && /^[A-Z0-9/]+$/.test(token) && /\d/.test(token) && /[A-Z]/.test(token) && !/^[A-R]{2}\d{2}$/.test(token);

const CQ_WORDS = ['CQ', 'CQDX', 'QRZ'];

/** Nominativo di chi trasmette: dopo il CQ (saltando i modificatori) o il secondo del messaggio. */
export function callOf(text: string): string | null {
  const tokens = text.trim().toUpperCase().split(/\s+/);
  if (CQ_WORDS.includes(tokens[0] ?? '')) {
    for (const token of tokens.slice(1)) {
      if (isCallToken(token)) return token;
      if (!/^[A-Z]{2,4}$/.test(token)) break;
    }
    return null;
  }
  const second = tokens[1];
  return second && isCallToken(second) ? second : null;
}

const isCq = (text: string) => CQ_WORDS.includes(text.trim().toUpperCase().split(/\s+/)[0] ?? '');

function isStandard(text: string): boolean {
  if (text.includes('<')) return false;
  const tokens = text.trim().toUpperCase().split(/\s+/);
  if (CQ_WORDS.includes(tokens[0] ?? '')) return callOf(text) !== null;
  return tokens.length >= 2 && isCallToken(tokens[0] ?? '') && isCallToken(tokens[1] ?? '');
}

function callMatches(call: string | null, pattern: string): boolean {
  if (!call) return false;
  const p = pattern.trim().toUpperCase();
  return p.endsWith('*') ? call.startsWith(p.slice(0, -1)) : call === p;
}

function signoff(text: string, strict: boolean): boolean {
  const last = text.trim().toUpperCase().split(/\s+/).at(-1) ?? '';
  return last === 'RR73' || last === '73' || (!strict && last === 'RRR');
}

export function applyFilters(decodes: DecodedMessage[], filters: Filters, ownContinent: string | null | undefined): DecodedMessage[] {
  const countries = filters.hideCountries.map((c) => c.trim().toLowerCase());
  return decodes.filter((d) => {
    if (d.continent && filters.hideContinents.includes(d.continent)) return false;
    if (filters.hideOwnContinent && ownContinent && d.continent === ownContinent) return false;
    if (d.country && countries.includes(d.country.toLowerCase())) return false;
    if (filters.hideCalls.length > 0) {
      const call = callOf(d.text);
      if (filters.hideCalls.some((p) => callMatches(call, p))) return false;
    }
    if (filters.only === 'cq' && !isCq(d.text)) return false;
    if (filters.only === 'cq_rrr' && !isCq(d.text) && !signoff(d.text, false)) return false;
    if (filters.only === 'cq_rr73' && !isCq(d.text) && !signoff(d.text, true)) return false;
    if (filters.hideNonStandard && !isStandard(d.text)) return false;
    if (filters.hideWorked && d.tag === 'worked_call' && isCq(d.text)) return false;
    return true;
  });
}

const strings = (value: unknown): string[] => (Array.isArray(value) ? value.filter((v): v is string => typeof v === 'string') : []);

export function readFilters(storage: ReadableStorage): Filters {
  const filters = defaultFilters();
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return filters;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    filters.hideContinents = strings(parsed.hideContinents).filter((c) => (CONTINENTS as readonly string[]).includes(c));
    filters.hideCountries = strings(parsed.hideCountries);
    filters.hideCalls = strings(parsed.hideCalls);
    if (typeof parsed.only === 'string' && ONLY.includes(parsed.only)) filters.only = parsed.only as OnlyMode;
    filters.hideNonStandard = parsed.hideNonStandard === true;
    filters.hideOwnContinent = parsed.hideOwnContinent === true;
    filters.hideWorked = parsed.hideWorked === true;
  } catch {
    return defaultFilters();
  }
  return filters;
}

export function saveFilters(storage: WritableStorage, filters: Filters): void {
  try {
    storage?.setItem(KEY, JSON.stringify(filters));
  } catch {
    // senza archivio i filtri non vengono ricordati, ma l'interfaccia funziona lo stesso
  }
}

interface FiltersState {
  filters: Filters;
  set(change: Partial<Filters>): void;
  reset(): void;
}

/** Stato condiviso: cambiare un filtro nelle impostazioni aggiorna subito gli elenchi. */
export const useFilters = create<FiltersState>((set, get) => ({
  filters: readFilters(browserStorage()),
  set(change) {
    const next = { ...get().filters, ...change };
    saveFilters(browserStorage(), next);
    set({ filters: next });
  },
  reset() {
    saveFilters(browserStorage(), defaultFilters());
    set({ filters: defaultFilters() });
  },
}));
