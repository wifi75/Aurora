import { create } from 'zustand';
import { putBandFilters } from '../api/client';
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
  /** Con `hideWorked`: nasconde anche gli scambi delle stazioni gia' collegate, non solo i loro CQ («Hide B4» di WSJT-X). */
  hideWorkedAll: boolean;
  /** Mostra soltanto i messaggi di questi nominativi o prefissi (JA*); vuoto = tutti (la «Whitelist» di WSJT-X). */
  showOnlyCalls: string[];
  /** Nominativi o prefissi sempre visibili, anche se un altro filtro li nasconderebbe («Always Pass» di WSJT-X). */
  alwaysShowCalls: string[];
  /** Nominativi o prefissi da evidenziare nell'elenco («Highlight Whitelist entries» di WSJT-X). */
  highlightCalls: string[];
  /** Paesi da evidenziare nell'elenco («Highlight Territory» di WSJT-X). */
  highlightCountries: string[];
  /** Sospende tutti i filtri senza perderne le impostazioni (il «Bypass» di WSJT-X). */
  bypass: boolean;
}

const KEY = 'aurora-filters';
const ONLY: readonly string[] = ['all', 'cq', 'cq_rrr', 'cq_rr73'];

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

export function defaultFilters(): Filters {
  return { hideContinents: [], hideCountries: [], hideCalls: [], only: 'all', hideNonStandard: false, hideOwnContinent: false, hideWorked: false, hideWorkedAll: false, showOnlyCalls: [], alwaysShowCalls: [], highlightCalls: [], highlightCountries: [], bypass: false };
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

export const isCq = (text: string) => CQ_WORDS.includes(text.trim().toUpperCase().split(/\s+/)[0] ?? '');

function isStandard(text: string): boolean {
  if (text.includes('<')) return false;
  const tokens = text.trim().toUpperCase().split(/\s+/);
  if (CQ_WORDS.includes(tokens[0] ?? '')) return callOf(text) !== null;
  return tokens.length >= 2 && isCallToken(tokens[0] ?? '') && isCallToken(tokens[1] ?? '');
}

export function callMatches(call: string | null, pattern: string): boolean {
  if (!call) return false;
  const p = pattern.trim().toUpperCase();
  return p.endsWith('*') ? call.startsWith(p.slice(0, -1)) : call === p;
}

function signoff(text: string, strict: boolean): boolean {
  const last = text.trim().toUpperCase().split(/\s+/).at(-1) ?? '';
  return last === 'RR73' || last === '73' || (!strict && last === 'RRR');
}

/**
 * Filtri ed evidenziazioni per nominativo, prefisso, paese e «gia' collegato» confrontati con WSJT-X 3.3
 * (`MessageFilterLogic` richiamata da `widgets/mainwindow.cpp`, impostazioni in `Configuration.cpp`, GPL-3.0):
 * Blacklist = `hideCalls`, Whitelist = `showOnlyCalls`, Always Pass = `alwaysShowCalls`, Hide B4 = `hideWorked`/`hideWorkedAll`,
 * Highlight Whitelist entries = `highlightCalls`, Highlight Territory = `highlightCountries`, Bypass = `bypass`.
 */
const anyMatch = (call: string | null, patterns: string[]) => patterns.some((p) => p.trim() !== '' && callMatches(call, p));

/** Vero se la riga va evidenziata per nominativo/prefisso o per paese (vale anche con il bypass, come in WSJT-X per la Whitelist). */
export function isFilterHighlight(d: Pick<DecodedMessage, 'text' | 'country'>, filters: Filters): boolean {
  if (filters.highlightCalls.length > 0 && anyMatch(callOf(d.text), filters.highlightCalls)) return true;
  if (filters.highlightCountries.length > 0 && d.country) {
    const country = d.country.toLowerCase();
    return filters.highlightCountries.some((c) => c.trim() !== '' && c.trim().toLowerCase() === country);
  }
  return false;
}

export function applyFilters(decodes: DecodedMessage[], filters: Filters, ownContinent: string | null | undefined): DecodedMessage[] {
  if (filters.bypass) return decodes;
  const countries = filters.hideCountries.map((c) => c.trim().toLowerCase());
  return decodes.filter((d) => {
    if (filters.alwaysShowCalls.length > 0 && anyMatch(callOf(d.text), filters.alwaysShowCalls)) return true;
    if (filters.showOnlyCalls.length > 0 && !anyMatch(callOf(d.text), filters.showOnlyCalls)) return false;
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
    if (filters.hideWorked && d.tag === 'worked_call' && (filters.hideWorkedAll || isCq(d.text))) return false;
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
    filters.hideWorkedAll = parsed.hideWorkedAll === true;
    filters.showOnlyCalls = strings(parsed.showOnlyCalls);
    filters.alwaysShowCalls = strings(parsed.alwaysShowCalls);
    filters.highlightCalls = strings(parsed.highlightCalls);
    filters.highlightCountries = strings(parsed.highlightCountries);
    filters.bypass = parsed.bypass === true;
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
    // anche le Automazioni seguono i filtri di Attivita' di banda
    void putBandFilters(next);
  },
  reset() {
    saveFilters(browserStorage(), defaultFilters());
    set({ filters: defaultFilters() });
    void putBandFilters(defaultFilters());
  },
}));
