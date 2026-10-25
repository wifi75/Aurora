import type { DecodedMessage, DecodeTag } from '../api/schemas';
import { isCallToMe } from './mycall';
import type { NotifyView } from './notifyView';

export type AlertKind = 'dxcc' | 'news' | 'me' | 'wanted';

/** I suoni tra cui scegliere per gli avvisi («Play alert sound»). */
export const SOUND_NAMES = ['chime', 'bell', 'pip', 'horn'] as const;
export type SoundName = (typeof SOUND_NAMES)[number];

export interface AlertPrefs {
  enabled: boolean;
  /** Da 0 a 1. */
  volume: number;
  /** Il suono degli avvisi di «Play alert sound». */
  sound: SoundName;
  /** Non suonare mentre si trasmette (di base si': un suono in cuffia o nel microfono durante il Tx disturba). */
  muteWhileTx: boolean;
}

const KEY = 'aurora-alerts';
const DEFAULTS: AlertPrefs = { enabled: false, volume: 0.5, sound: 'chime', muteWhileTx: true };

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

/** Un paese nuovo suona forte; le altre novita' (nuova banda, zona, griglia) con una nota sola. Il CQ semplice tace. */
export function alertKind(tag: DecodeTag | null | undefined, sound?: Partial<Record<DecodeTag, boolean>>): AlertKind | null {
  if (tag && sound && sound[tag] === false) return null;
  if (tag && sound && sound[tag] === true && alertKind(tag) === null && tag !== 'auto') return 'news';
  switch (tag) {
    case 'new_dxcc':
      return 'dxcc';
    case 'new_dxcc_band':
    case 'new_zone':
    case 'new_zone_band':
    case 'new_itu':
    case 'new_grid':
    case 'new_px':
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
    const { enabled, volume, sound, muteWhileTx } = parsed as Record<string, unknown>;
    return {
      enabled: enabled === true,
      volume: typeof volume === 'number' && Number.isFinite(volume) ? Math.min(1, Math.max(0, volume)) : DEFAULTS.volume,
      sound: SOUND_NAMES.find((n) => n === sound) ?? DEFAULTS.sound,
      muteWhileTx: typeof muteWhileTx === 'boolean' ? muteWhileTx : DEFAULTS.muteWhileTx,
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
export function pickAlert(
  decodes: DecodedMessage[],
  lastId: number | null,
  sound?: Partial<Record<string, boolean>>,
  myCall?: string | null,
  firstMessage = false,
  custom?: { flags: Pick<NotifyView, AlertFlagKey>; dxCall?: string | null },
): { kind: AlertKind | null; lastId: number | null; alert?: boolean } {
  const newest = decodes.reduce<number | null>((max, d) => (max === null || d.id > max ? d.id : max), null);
  if (lastId === null) return { kind: null, lastId: newest };
  let kind: AlertKind | null = null;
  for (const d of decodes) {
    if (d.id <= lastId) continue;
    // una stazione che mi chiama batte qualunque novita'
    if (myCall && sound?.my_call !== false && isCallToMe(d.text, myCall)) {
      kind = 'me';
      break;
    }
    const found = alertKind(d.tag, sound);
    if (found === 'dxcc') {
      kind = 'dxcc';
      break;
    }
    if (found) kind = found;
  }
  // «Suono al primo messaggio decodificato»: se arriva qualcosa di nuovo (non nostro) e niente di meglio ha suonato, una nota
  if (firstMessage && kind === null && decodes.some((d) => d.id > lastId && d.id >= 0)) kind = 'news';
  const alert = custom ? alertFor(decodes, lastId, custom.flags, myCall, custom.dxCall) : false;
  return { kind, lastId: newest === null ? lastId : Math.max(lastId, newest), ...(custom ? { alert } : {}) };
}

/** Le chiavi dei flag degli avvisi. */
export type AlertFlagKey = 'alertMyCall' | 'alertDxCall' | 'alertCqz' | 'alertItu' | 'alertDxcc' | 'alertGrid' | 'alertPx' | 'alertCqzOb' | 'alertItuOb' | 'alertDxccOb' | 'alertGridOb' | 'alertPxOb';

/** Quale flag di JTDX accende l'avviso per un tipo di messaggio: il «nuovo» e, con «per banda», il «nuovo su banda/modo». */
const ALERT_OF_TAG: Partial<Record<DecodeTag, AlertFlagKey>> = {
  new_zone: 'alertCqz',
  new_zone_band: 'alertCqzOb',
  new_itu: 'alertItu',
  new_itu_band: 'alertItuOb',
  new_dxcc: 'alertDxcc',
  new_dxcc_band: 'alertDxccOb',
  new_grid: 'alertGrid',
  new_grid_band: 'alertGridOb',
  new_px: 'alertPx',
  new_px_band: 'alertPxOb',
};

/** «Play alert sound»: un messaggio nuovo (non nostro) che chiama me, nomina il DX scelto o porta una novita' con il suo avviso acceso. */
function alertFor(decodes: DecodedMessage[], lastId: number, flags: Pick<NotifyView, AlertFlagKey>, myCall?: string | null, dxCall?: string | null): boolean {
  const dx = dxCall?.trim().toUpperCase() ?? '';
  for (const d of decodes) {
    if (d.id <= lastId || d.id < 0) continue;
    if (flags.alertMyCall && myCall && isCallToMe(d.text, myCall)) return true;
    if (flags.alertDxCall && dx !== '' && d.text.toUpperCase().split(/\s+/).includes(dx)) return true;
    const key = d.tag ? ALERT_OF_TAG[d.tag] : undefined;
    if (key && flags[key]) return true;
  }
  return false;
}

/** Quello che serve per fare un suono: separato da WebAudio cosi' si prova senza scheda audio. */
export interface ToneContext {
  now(): number;
  tone(hz: number, gain: number, at: number, seconds: number): void;
}

const NOTES: Record<AlertKind, number[]> = {
  dxcc: [660, 880, 1320],
  news: [880],
  me: [1047, 1319, 1047, 1319],
  // una stazione della lista dei desiderati: due note che salgono, poi ancora una piu' acuta
  wanted: [784, 1175, 1568, 1976],
};

/** Gli avvisi con un suono a scelta: note diverse per ognuno. */
const NAMED: Record<SoundName, { notes: number[]; step: number; length: number }> = {
  chime: { notes: [988, 1319, 1568], step: 0.16, length: 0.22 },
  bell: { notes: [880, 880, 660], step: 0.3, length: 0.5 },
  pip: { notes: [1800, 1800], step: 0.12, length: 0.06 },
  horn: { notes: [330, 330, 392], step: 0.25, length: 0.22 },
};

/** Il suono scelto per gli avvisi di «Play alert sound» (e il pulsante «Prova»). */
export function playNamed(name: SoundName, volume: number, ctx: ToneContext): void {
  const gain = Math.min(1, Math.max(0, volume)) * 0.5;
  if (gain <= 0) return;
  const { notes, step, length } = NAMED[name];
  const start = ctx.now();
  notes.forEach((hz, i) => ctx.tone(hz, gain, start + i * step, length));
}

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
