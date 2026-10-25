import { create } from 'zustand';
import type { RadioMeters } from '../api/schemas';
import { browserStorage } from './theme';

/** Gli stili del display della radio, nell'ordine in cui si propongono nelle Impostazioni. */
export const RADIO_DISPLAY_STYLES = ['fluorescent', 'analog', 'color', 'amber', 'led'] as const;
export type RadioDisplayStyle = (typeof RADIO_DISPLAY_STYLES)[number];

export interface RadioDisplayPref {
  style: RadioDisplayStyle;
  /** Display nascosto: lo stile scelto resta ricordato. */
  hidden: boolean;
}

export const DEFAULT_RADIO_DISPLAY: RadioDisplayPref = { style: 'fluorescent', hidden: false };

const KEY = 'aurora-radio-display';

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

export function isRadioDisplayStyle(value: unknown): value is RadioDisplayStyle {
  return typeof value === 'string' && (RADIO_DISPLAY_STYLES as readonly string[]).includes(value);
}

/** La scelta salvata; un valore illeggibile o sconosciuto riporta al predefinito. */
export function readRadioDisplay(storage: ReadableStorage): RadioDisplayPref {
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return { ...DEFAULT_RADIO_DISPLAY };
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    return {
      style: isRadioDisplayStyle(parsed.style) ? parsed.style : DEFAULT_RADIO_DISPLAY.style,
      hidden: parsed.hidden === true,
    };
  } catch {
    return { ...DEFAULT_RADIO_DISPLAY };
  }
}

export function saveRadioDisplay(storage: WritableStorage, pref: RadioDisplayPref): void {
  try {
    storage?.setItem(KEY, JSON.stringify(pref));
  } catch {
    // senza archivio la scelta non viene ricordata, ma il display funziona lo stesso
  }
}

interface RadioDisplayState {
  pref: RadioDisplayPref;
  set(change: Partial<RadioDisplayPref>): void;
}

/** Lo stile del display, condiviso dalla testata e dalle Impostazioni; ogni cambio si salva subito. */
export const useRadioDisplay = create<RadioDisplayState>((set, get) => ({
  pref: readRadioDisplay(browserStorage()),
  set(change) {
    const pref = { ...get().pref, ...change };
    saveRadioDisplay(browserStorage(), pref);
    set({ pref });
  },
}));

/**
 * Frequenza come sul display di una radio: MHz.kHz.centinaia di Hz (`21.074.00`) oppure con gli Hz interi (`21.074.000`).
 * Con una frequenza nulla o sconosciuta: trattini al posto delle cifre.
 */
export function formatRadioFrequency(hz: number, precision: 'tens' | 'hz' = 'tens'): string {
  if (!Number.isFinite(hz) || hz <= 0) return precision === 'hz' ? '--.---.---' : '--.---.--';
  const whole = Math.round(hz);
  const mhz = Math.floor(whole / 1_000_000);
  const khz = Math.floor((whole % 1_000_000) / 1000);
  const rest = whole % 1000;
  const tail = precision === 'hz' ? String(rest).padStart(3, '0') : String(Math.floor(rest / 10)).padStart(2, '0');
  return `${mhz}.${String(khz).padStart(3, '0')}.${tail}`;
}

/** Vero se la radio ha dato una frequenza utilizzabile. */
export function hasFrequency(hz: number | undefined | null): boolean {
  return typeof hz === 'number' && Number.isFinite(hz) && hz > 0;
}

const clamp01 = (n: number) => Math.min(1, Math.max(0, n));

/** Punto della scala S in cui cade S9: da li' in poi la scala conta i decibel. */
export const S9_FRACTION = 0.6;
/** Fondo scala della potenza (W) e della compressione (dB): le scale sono lineari fino a qui. */
export const PO_MAX_W = 100;
export const COMP_MAX_DB = 30;

/** Posizione (0-1) sulla scala S; `null` se la radio non da' l'S-meter (la scala resta spenta). */
export function sFraction(m: RadioMeters | null | undefined): number | null {
  if (!m) return null;
  const { s_points: s, s_over_db: over } = m;
  if (s === undefined && over === undefined) return null;
  const points = s ?? 9;
  const overDb = over ?? 0;
  if (points < 9 && overDb <= 0) return clamp01(points / 9) * S9_FRACTION;
  return clamp01(S9_FRACTION + clamp01(overDb / 60) * (1 - S9_FRACTION));
}

/** Lettura dell'S-meter senza contrassegni: `S7`, `S9+20`; `null` senza dato. */
export function sValue(m: RadioMeters | null | undefined): string | null {
  if (sFraction(m) === null || !m) return null;
  const points = Math.round(m.s_points ?? 9);
  const over = Math.round(m.s_over_db ?? 0);
  if (points >= 9 && over > 0) return `S9+${over}`;
  return `S${Math.max(0, Math.min(9, points))}`;
}

/** L'S-meter e' una stima dal livello dell'audio, non una lettura della radio. */
export function sIsEstimated(m: RadioMeters | null | undefined): boolean {
  return m?.estimated === true && sFraction(m) !== null;
}

/** Lettura dell'S-meter per il disegno: `S7`, `S9+20`, con la tilde davanti se e' stimato (`~S7`); `null` senza dato. */
export function sLabel(m: RadioMeters | null | undefined): string | null {
  const value = sValue(m);
  return value !== null && sIsEstimated(m) ? `~${value}` : value;
}

export function powerFraction(m: RadioMeters | null | undefined): number | null {
  return typeof m?.power_w === 'number' ? clamp01(m.power_w / PO_MAX_W) : null;
}

// ROS: la scala dei radioamatori non e' lineare (1, 1,5, 2, 3, oltre)
const SWR_POINTS: ReadonlyArray<readonly [number, number]> = [
  [1, 0],
  [1.5, 0.25],
  [2, 0.5],
  [3, 0.75],
  [5, 1],
];

export function swrFraction(m: RadioMeters | null | undefined): number | null {
  const swr = m?.swr;
  if (typeof swr !== 'number' || !Number.isFinite(swr)) return null;
  if (swr <= 1) return 0;
  for (let i = 1; i < SWR_POINTS.length; i++) {
    const [x1, y1] = SWR_POINTS[i] ?? [5, 1];
    const [x0, y0] = SWR_POINTS[i - 1] ?? [1, 0];
    if (swr <= x1) return y0 + ((swr - x0) / (x1 - x0)) * (y1 - y0);
  }
  return 1;
}

export function alcFraction(m: RadioMeters | null | undefined): number | null {
  return typeof m?.alc === 'number' ? clamp01(m.alc) : null;
}

export function compFraction(m: RadioMeters | null | undefined): number | null {
  return typeof m?.comp_db === 'number' ? clamp01(m.comp_db / COMP_MAX_DB) : null;
}

/** Quante delle `count` tacche di una scala restano accese; 0 con la scala spenta. */
export function litCount(fraction: number | null, count: number): number {
  return fraction === null ? 0 : Math.round(clamp01(fraction) * count);
}

/** I dati che il display disegna, ricavati dallo stato della stazione. */
export interface DisplayModel {
  dialHz: number;
  mode: string;
  transmitting: boolean;
  decoding: boolean;
  txEnabled: boolean;
  meters: RadioMeters | null;
}

interface StatusLike {
  dial_hz: number;
  mode: string;
  transmitting: boolean;
  decoding: boolean;
  tx_enabled: boolean;
  meters?: RadioMeters | null;
}

export function displayModel(status: StatusLike | null | undefined): DisplayModel {
  return {
    dialHz: status?.dial_hz ?? 0,
    mode: status?.mode ?? '',
    transmitting: status?.transmitting ?? false,
    decoding: status?.decoding ?? false,
    txEnabled: status?.tx_enabled ?? false,
    meters: status?.meters ?? null,
  };
}

/** Dati di prova per le anteprime nelle Impostazioni. */
export const DEMO_MODEL: DisplayModel = {
  dialHz: 21_074_000,
  mode: 'FT8',
  transmitting: false,
  decoding: true,
  txEnabled: false,
  meters: { s_points: 7, s_over_db: 0, power_w: 25, swr: 1.4, alc: 0.35, comp_db: 8, id_a: 3.2, vdd_v: 13.8 },
};

/** Tacche della scala S: etichetta e posizione (0-1), come sulle radio. */
export const S_TICKS: ReadonlyArray<{ label: string; fraction: number }> = [
  { label: '1', fraction: (1 / 9) * S9_FRACTION },
  { label: '3', fraction: (3 / 9) * S9_FRACTION },
  { label: '5', fraction: (5 / 9) * S9_FRACTION },
  { label: '7', fraction: (7 / 9) * S9_FRACTION },
  { label: '9', fraction: S9_FRACTION },
  { label: '+20', fraction: S9_FRACTION + (20 / 60) * (1 - S9_FRACTION) },
  { label: '+40', fraction: S9_FRACTION + (40 / 60) * (1 - S9_FRACTION) },
  { label: '+60', fraction: 1 },
];

export interface MainMeter {
  kind: 'S' | 'PO';
  /** Posizione 0-1; `null` se la radio non da' lo strumento. */
  fraction: number | null;
  /** Lettura a parole (`S7`, `25 W`); `null` senza dato. */
  label: string | null;
  /** S-meter stimato dall'audio (il disegno mostra il contrassegno «stima»); sempre falso per la potenza. */
  estimated: boolean;
}

const trimNumber = (n: number, digits: number) => String(Number(n.toFixed(digits)));

/** Lo strumento principale: S-meter in ricezione, potenza in trasmissione (come sulle radio). */
export function mainMeter(model: DisplayModel): MainMeter {
  if (model.transmitting) {
    const fraction = powerFraction(model.meters);
    const w = model.meters?.power_w;
    return { kind: 'PO', fraction, label: fraction === null || w === undefined ? null : `${trimNumber(w, w < 10 ? 1 : 0)} W`, estimated: false };
  }
  return { kind: 'S', fraction: sFraction(model.meters), label: sLabel(model.meters), estimated: sIsEstimated(model.meters) };
}

/** Le letture numeriche presenti (solo quelle che la radio da'), per le righe di testo dei display grandi. */
export function meterReadouts(m: RadioMeters | null | undefined): Array<{ key: 'PO' | 'SWR' | 'ALC' | 'COMP' | 'Id' | 'Vdd'; text: string }> {
  const out: Array<{ key: 'PO' | 'SWR' | 'ALC' | 'COMP' | 'Id' | 'Vdd'; text: string }> = [];
  if (!m) return out;
  if (m.power_w !== undefined) out.push({ key: 'PO', text: `${trimNumber(m.power_w, m.power_w < 10 ? 1 : 0)} W` });
  if (m.swr !== undefined) out.push({ key: 'SWR', text: trimNumber(m.swr, 1) });
  if (m.alc !== undefined) out.push({ key: 'ALC', text: `${Math.round(clamp01(m.alc) * 100)} %` });
  if (m.comp_db !== undefined) out.push({ key: 'COMP', text: `${trimNumber(m.comp_db, 0)} dB` });
  if (m.id_a !== undefined) out.push({ key: 'Id', text: `${trimNumber(m.id_a, 1)} A` });
  if (m.vdd_v !== undefined) out.push({ key: 'Vdd', text: `${trimNumber(m.vdd_v, 1)} V` });
  return out;
}
