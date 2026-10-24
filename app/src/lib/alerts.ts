import type { DecodedMessage, DecodeTag } from '../api/schemas';

export type AlertKind = 'dxcc' | 'news';

export interface AlertPrefs {
  enabled: boolean;
  /** Da 0 a 1. */
  volume: number;
}

const KEY = 'aurora-alerts';
const DEFAULTS: AlertPrefs = { enabled: false, volume: 0.5 };

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

/** Un paese nuovo suona forte; le altre novita' (nuova banda, zona, griglia) con una nota sola. Il CQ semplice tace. */
export function alertKind(tag: DecodeTag | null | undefined): AlertKind | null {
  switch (tag) {
    case 'new_dxcc':
      return 'dxcc';
    case 'new_dxcc_band':
    case 'new_zone':
    case 'new_grid':
      return 'news';
    default:
      return null;
  }
}

/** Gli avvisi partono spenti: un suono mai richiesto in una sala radio e' peggio di nessun suono. */
export function readAlerts(storage: ReadableStorage): AlertPrefs {
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return { ...DEFAULTS };
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== 'object' || parsed === null) return { ...DEFAULTS };
    const { enabled, volume } = parsed as Record<string, unknown>;
    return {
      enabled: enabled === true,
      volume: typeof volume === 'number' && Number.isFinite(volume) ? Math.min(1, Math.max(0, volume)) : DEFAULTS.volume,
    };
  } catch {
    return { ...DEFAULTS };
  }
}

export function saveAlerts(storage: WritableStorage, prefs: AlertPrefs): void {
  try {
    storage?.setItem(KEY, JSON.stringify(prefs));
  } catch {
    // senza archivio la scelta non viene ricordata, ma gli avvisi funzionano lo stesso
  }
}

/**
 * Tra i messaggi arrivati dopo `lastId` (i piu' recenti per primi) sceglie l'avviso piu' importante.
 * Alla prima chiamata (`lastId` nullo) si limita a ricordare dove siamo: i messaggi gia' presenti non suonano.
 */
export function pickAlert(decodes: DecodedMessage[], lastId: number | null): { kind: AlertKind | null; lastId: number | null } {
  const newest = decodes.reduce<number | null>((max, d) => (max === null || d.id > max ? d.id : max), null);
  if (lastId === null) return { kind: null, lastId: newest };
  let kind: AlertKind | null = null;
  for (const d of decodes) {
    if (d.id <= lastId) continue;
    const found = alertKind(d.tag);
    if (found === 'dxcc') {
      kind = 'dxcc';
      break;
    }
    if (found) kind = found;
  }
  return { kind, lastId: newest === null ? lastId : Math.max(lastId, newest) };
}

/** Quello che serve per fare un suono: separato da WebAudio cosi' si prova senza scheda audio. */
export interface ToneContext {
  now(): number;
  tone(hz: number, gain: number, at: number, seconds: number): void;
}

const NOTES: Record<AlertKind, number[]> = {
  dxcc: [660, 880, 1320],
  news: [880],
};

export function playAlert(kind: AlertKind, volume: number, ctx: ToneContext): void {
  const gain = Math.min(1, Math.max(0, volume)) * 0.5;
  if (gain <= 0) return;
  const start = ctx.now();
  NOTES[kind].forEach((hz, i) => ctx.tone(hz, gain, start + i * 0.14, 0.12));
}

/** Contesto reale su WebAudio; `null` se il browser non lo offre. Va creato dopo un gesto dell'utente. */
export function webAudioContext(): ToneContext | null {
  const Ctor: typeof AudioContext | undefined =
    typeof window === 'undefined' ? undefined : (window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext);
  if (!Ctor) return null;
  const audio = new Ctor();
  return {
    now: () => audio.currentTime,
    tone(hz, gain, at, seconds) {
      void audio.resume();
      const osc = audio.createOscillator();
      const amp = audio.createGain();
      osc.type = 'sine';
      osc.frequency.value = hz;
      amp.gain.setValueAtTime(0, at);
      amp.gain.linearRampToValueAtTime(gain, at + 0.01);
      amp.gain.linearRampToValueAtTime(0, at + seconds);
      osc.connect(amp).connect(audio.destination);
      osc.start(at);
      osc.stop(at + seconds + 0.02);
    },
  };
}
