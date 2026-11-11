import { create } from 'zustand';
import { browserStorage } from './theme';

/**
 * «Segna chi usa LoTW» (issue #36, EA4HOS): sotto il nominativo di chi trasmette un messaggio c'e' una barra colorata se la stazione ha
 * mandato log a LoTW da non piu' di N mesi. Il motore manda con ogni decodifica la data dell'ultimo invio (`lotw_last_upload`, dall'elenco
 * pubblico dell'ARRL); qui si decide se conta, in base a questa scelta, che e' dell'interfaccia e si ricorda per dispositivo.
 */
export interface LotwView {
  /** Acceso di fabbrica. */
  enabled: boolean;
  /** Colore della barra (verde chiaro di fabbrica). */
  color: string;
  /** Da quanti mesi al massimo e' stato l'ultimo invio: 0 = qualunque data. */
  months: number;
}

/** Le scelte del menu dei mesi (0 = «qualunque»). */
export const LOTW_MONTHS = [3, 6, 12, 24, 0] as const;

const KEY = 'aurora-lotw';
const HEX = /^#[0-9a-fA-F]{6}$/;

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

export function defaultLotwView(): LotwView {
  return { enabled: true, color: '#5fd35f', months: 12 };
}

export function readLotwView(storage: ReadableStorage): LotwView {
  const view = defaultLotwView();
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return view;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    if (typeof parsed.enabled === 'boolean') view.enabled = parsed.enabled;
    if (typeof parsed.color === 'string' && HEX.test(parsed.color)) view.color = parsed.color.toLowerCase();
    if (typeof parsed.months === 'number' && (LOTW_MONTHS as readonly number[]).includes(parsed.months)) view.months = parsed.months;
  } catch {
    return defaultLotwView();
  }
  return view;
}

export function saveLotwView(storage: WritableStorage, view: LotwView): void {
  try {
    storage?.setItem(KEY, JSON.stringify(view));
  } catch {
    // senza archivio le scelte non vengono ricordate, ma l'interfaccia funziona lo stesso
  }
}

/** Vero se `date` (`AAAA-MM-GG`) e' dentro gli ultimi `months` mesi da `now`; con 0 mesi vale qualunque data valida. */
export function lotwIsRecent(date: string | null | undefined, months: number, now: Date): boolean {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(date ?? '');
  if (!m) return false;
  if (months <= 0) return true;
  const last = Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
  const cutoff = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth() - months, now.getUTCDate()));
  return last >= cutoff.getTime();
}

/** La data dell'ultimo invio da mostrare, o `null` se la stazione non conta come utente di LoTW con queste scelte. */
export function lotwDateToShow(date: string | null | undefined, view: LotwView, now: Date): string | null {
  if (!view.enabled || !date) return null;
  return lotwIsRecent(date, view.months, now) ? date : null;
}

interface LotwViewState {
  view: LotwView;
  set(change: Partial<LotwView>): void;
}

export const useLotwView = create<LotwViewState>((set, get) => ({
  view: readLotwView(browserStorage()),
  set(change) {
    const next = { ...get().view, ...change };
    saveLotwView(browserStorage(), next);
    set({ view: next });
  },
}));
