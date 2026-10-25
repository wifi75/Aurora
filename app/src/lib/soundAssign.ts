import { create } from 'zustand';
import type { DecodedMessage, DecodeTag } from '../api/schemas';
import type { Key } from '../i18n';
import { ALERTS_KEY, readAlerts, useAlertPrefs } from './alerts';
import { senderOf } from './wishlist';
import { type ColorKey } from './colors';
import { isCallToMe } from './mycall';
import { readNotify, useNotify } from './notify';
import { readNotifyView, useNotifyView, type AlertKey, type NotifyView } from './notifyView';
import { hasSound } from './soundBank';
import { browserStorage } from './theme';

/**
 * Quale suono suona per ogni funzione. Una funzione suona solo se (a) il suo avviso e' acceso come prima («anche un suono»,
 * «Riproduci un suono di avviso», «suono al primo messaggio») e (b) ha un suono assegnato; senza suono assegnato tace.
 */

/** Le funzioni a cui si assegna un suono: i tipi di messaggio con colore e gli avvisi che c'erano gia'. */
export const SOUND_EVENTS = [
  'tx_msg',
  'cq',
  'my_call',
  'standard_call',
  'new_zone',
  'new_zone_band',
  'new_itu',
  'new_itu_band',
  'new_dxcc',
  'new_dxcc_band',
  'new_grid',
  'new_grid_band',
  'new_px',
  'new_px_band',
  'new_call',
  'new_call_band',
  'worked_call',
  'alert_my_call',
  'alert_dx_call',
  'first_message',
  'wanted',
] as const;
export type SoundEvent = (typeof SOUND_EVENTS)[number];
export type Assign = Record<SoundEvent, string | null>;

/** Le funzioni che non sono un tipo di messaggio con colore: il loro nome sta nei testi `snd.ev.*`. */
export const EXTRA_EVENTS = ['alert_my_call', 'alert_dx_call', 'first_message', 'wanted'] as const satisfies readonly SoundEvent[];
export type ExtraEvent = (typeof EXTRA_EVENTS)[number];

export function isTagEvent(event: SoundEvent): event is Extract<SoundEvent, ColorKey> {
  return !(EXTRA_EVENTS as readonly string[]).includes(event);
}

export function eventNameKey(event: SoundEvent): Key {
  return (isTagEvent(event) ? `kind.${event}` : `snd.ev.${event}`) as Key;
}

/** Il suono che c'era prima per ogni funzione: serve alla migrazione e a «acceso senza suono scelto». */
export function legacySound(event: SoundEvent, alertSound: string): string {
  switch (event) {
    case 'my_call':
      return 'trill4';
    case 'new_dxcc':
      return 'rise3';
    case 'wanted':
      return 'rise4';
    case 'alert_my_call':
    case 'alert_dx_call':
      return alertSound;
    default:
      return 'single';
  }
}

/** «Assegna suoni diversi automaticamente»: un suono diverso per ogni funzione, scelto perche' le piu' importanti si distinguano a orecchio. */
export const AUTO_SOUNDS: Record<SoundEvent, string> = {
  tx_msg: 'tictoc',
  cq: 'pip',
  my_call: 'trill4',
  standard_call: 'click2',
  new_zone: 'xylo',
  new_zone_band: 'pizz',
  new_itu: 'arp',
  new_itu_band: 'gliss',
  new_dxcc: 'fanfare',
  new_dxcc_band: 'carillon',
  new_grid: 'drops',
  new_grid_band: 'bubbles',
  new_px: 'dingdong',
  new_px_band: 'doorbell',
  new_call: 'chime',
  new_call_band: 'beep2',
  worked_call: 'single',
  alert_my_call: 'siren',
  alert_dx_call: 'cwdx',
  first_message: 'chirp',
  wanted: 'rise4',
};

export function emptyAssign(): Assign {
  return Object.fromEntries(SOUND_EVENTS.map((e) => [e, null])) as Assign;
}

/** Quale avviso di «Play alert sound» e' legato a un tipo di messaggio: il «nuovo» e, con «per banda», il «nuovo su banda/modo». */
const ALERT_OF_TAG: Partial<Record<string, AlertKey>> = {
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

/** La funzione a cui appartiene un avviso di «Play alert sound». */
export function eventOfAlert(key: AlertKey): SoundEvent {
  if (key === 'alertMyCall') return 'alert_my_call';
  if (key === 'alertDxCall') return 'alert_dx_call';
  return (Object.entries(ALERT_OF_TAG).find(([, k]) => k === key)?.[0] ?? 'new_dxcc') as SoundEvent;
}

type GateView = Pick<NotifyView, AlertKey | 'beepFirst'>;

/** (a) L'avviso della funzione e' acceso come prima? */
export function gateOn(event: SoundEvent, sound: Partial<Record<string, boolean>>, view: GateView): boolean {
  switch (event) {
    case 'alert_my_call':
      return view.alertMyCall;
    case 'alert_dx_call':
      return view.alertDxCall;
    case 'first_message':
      return view.beepFirst;
    case 'wanted':
      return true;
    default: {
      const alert = ALERT_OF_TAG[event];
      return sound[event] === true || (alert !== undefined && view[alert]);
    }
  }
}

const STORAGE_KEY = 'aurora-sound-assign';
const LEGACY_KEYS = [ALERTS_KEY, 'aurora-notify', 'aurora-notify-view'];

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

/**
 * Migrazione: chi aveva un suono acceso per un tipo ottiene il suono standard di prima per quel tipo (gli avvisi di «Play alert sound» il
 * suono che avevano scelto); chi non aveva mai toccato niente non ha nessun suono assegnato.
 */
export function migrateLegacy(storage: ReadableStorage): Assign {
  const assign = emptyAssign();
  let any = false;
  try {
    any = LEGACY_KEYS.some((k) => storage?.getItem(k));
  } catch {
    any = false;
  }
  if (!any) return assign;
  const sound = readNotify(storage).sound;
  const view = readNotifyView(storage);
  const chosen = readAlerts(storage).sound;
  for (const event of SOUND_EVENTS) {
    if (!gateOn(event, sound, view)) continue;
    const alert = ALERT_OF_TAG[event];
    assign[event] = alert !== undefined && view[alert] ? chosen : legacySound(event, chosen);
  }
  return assign;
}

/** Quello che e' salvato; senza niente salvato, la migrazione delle vecchie preferenze. */
export function readAssign(storage: ReadableStorage): Assign {
  try {
    const raw = storage?.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as unknown;
      if (typeof parsed === 'object' && parsed !== null) {
        const map = (parsed as { map?: unknown }).map;
        const assign = emptyAssign();
        if (typeof map === 'object' && map !== null) {
          for (const event of SOUND_EVENTS) {
            const value = (map as Record<string, unknown>)[event];
            if (typeof value === 'string' && hasSound(value)) assign[event] = value;
          }
        }
        return assign;
      }
    }
  } catch {
    return emptyAssign();
  }
  return migrateLegacy(storage);
}

export function saveAssign(storage: WritableStorage, assign: Assign): void {
  try {
    storage?.setItem(STORAGE_KEY, JSON.stringify({ v: 1, map: assign }));
  } catch {
    // senza archivio la scelta non viene ricordata, ma i suoni funzionano lo stesso
  }
}

/** Accende l'avviso di una funzione (la casella «anche un suono», l'avviso, il suono al primo messaggio). */
export function enableGate(event: SoundEvent): void {
  switch (event) {
    case 'alert_my_call':
      useNotifyView.getState().set({ alertMyCall: true });
      break;
    case 'alert_dx_call':
      useNotifyView.getState().set({ alertDxCall: true });
      break;
    case 'first_message':
      useNotifyView.getState().set({ beepFirst: true });
      break;
    case 'wanted':
      break;
    default:
      useNotify.getState().setFlag('sound', event, true);
  }
}

interface AssignState {
  assign: Assign;
  /** Assegna un suono (o `null` = nessuno). Con un suono, accende anche l'avviso di quella funzione. */
  set(event: SoundEvent, id: string | null): void;
  setAll(next: Assign): void;
  clear(): void;
  /** Dopo aver acceso un avviso: se non ha un suono, gli da' quello di prima, cosi' accendere vuol dire che suona. */
  ensure(events: readonly SoundEvent[]): void;
}

function commit(next: Assign) {
  saveAssign(browserStorage(), next);
  return { assign: next };
}

export const useSoundAssign = create<AssignState>((set, get) => ({
  assign: (() => {
    const storage = browserStorage();
    const assign = readAssign(storage);
    // la migrazione si fa una volta e si ricorda
    try {
      if (!storage?.getItem(STORAGE_KEY)) saveAssign(storage, assign);
    } catch {
      // senza archivio si rifa' a ogni apertura, con lo stesso risultato
    }
    return assign;
  })(),
  set(event, id) {
    const value = id !== null && hasSound(id) ? id : null;
    if (value) enableGate(event);
    set(commit({ ...get().assign, [event]: value }));
  },
  setAll(next) {
    set(commit({ ...next }));
  },
  clear() {
    set(commit(emptyAssign()));
  },
  ensure(events) {
    const current = get().assign;
    const alertSound = useAlertPrefs.getState().prefs.sound;
    const next = { ...current };
    let changed = false;
    for (const e of events) {
      if (!next[e]) {
        next[e] = legacySound(e, alertSound);
        changed = true;
      }
    }
    if (changed) set(commit(next));
  },
}));

/** Tutto quello che serve a decidere se una funzione suona, senza toccare gli archivi (si prova con dati finti). */
export interface RingState {
  assign: Assign;
  sound: Partial<Record<string, boolean>>;
  view: GateView;
  enabled: boolean;
  muteWhileTx: boolean;
  transmitting: boolean;
  volume: number;
}

/** Lo stato attuale preso dagli archivi dell'interfaccia. */
export function ringState(transmitting: boolean): RingState {
  const prefs = useAlertPrefs.getState().prefs;
  return {
    assign: useSoundAssign.getState().assign,
    sound: useNotify.getState().notify.sound,
    view: useNotifyView.getState().view,
    enabled: prefs.enabled,
    muteWhileTx: prefs.muteWhileTx,
    transmitting,
    volume: prefs.volume,
  };
}

/**
 * Il suono da suonare per una funzione, o `null`: serve «Avvisi sonori» acceso, l'avviso della funzione acceso e un suono assegnato.
 * Mentre si trasmette tace (se cosi' scelto), tranne «mio messaggio in trasmissione», che e' proprio il segnale del Tx.
 */
export function soundFor(event: SoundEvent, st: RingState): string | null {
  if (!st.enabled) return null;
  if (st.muteWhileTx && st.transmitting && event !== 'tx_msg') return null;
  const id = st.assign[event];
  if (!hasSound(id)) return null;
  return gateOn(event, st.sound, st.view) ? id : null;
}

/** Con piu' funzioni nello stesso gruppo di messaggi suona la piu' importante. */
const PRIORITY: readonly SoundEvent[] = [
  'my_call',
  'alert_my_call',
  'alert_dx_call',
  'new_dxcc',
  'new_dxcc_band',
  'new_zone',
  'new_zone_band',
  'new_itu',
  'new_itu_band',
  'new_px',
  'new_px_band',
  'new_grid',
  'new_grid_band',
  'new_call',
  'new_call_band',
  'standard_call',
  'cq',
  'worked_call',
];

/** Le funzioni che un messaggio fa scattare. */
function eventsOf(d: DecodedMessage, myCall?: string | null, dxCall?: string | null): SoundEvent[] {
  const found: SoundEvent[] = [];
  if (myCall && isCallToMe(d.text, myCall)) found.push('my_call', 'alert_my_call');
  const dx = dxCall?.trim().toUpperCase() ?? '';
  if (dx !== '' && d.text.toUpperCase().split(/\s+/).includes(dx)) found.push('alert_dx_call');
  const tag: DecodeTag | null | undefined = d.tag;
  if (tag && (PRIORITY as readonly string[]).includes(tag)) found.push(tag as SoundEvent);
  return found;
}

/** Una stazione fa suonare una notifica una volta sola: poi non suona piu' per tutta la sessione (12 ore), anche se ritrasmette o cambia funzione. */
export const NOTIFIED_TTL_MS = 12 * 60 * 60_000;
const notifiedAt = new Map<string, number>();

/** Questa stazione ha gia' fatto suonare una notifica? Allora non suona di nuovo. */
function alreadyNotified(sender: string, now: number): boolean {
  const at = notifiedAt.get(sender);
  return at !== undefined && now - at < NOTIFIED_TTL_MS;
}

/** Per le prove e per il cambio di banda: si dimentica chi e' gia' stato segnalato. */
export function resetNotified(): void {
  notifiedAt.clear();
}

export interface Picked {
  event: SoundEvent | null;
  sound: string | null;
  lastId: number | null;
}

/**
 * Tra i messaggi arrivati dopo `lastId` sceglie la funzione piu' importante che deve suonare. Alla prima chiamata (`lastId` nullo)
 * ricorda dove siamo: i messaggi gia' presenti non suonano. Le righe Tx (identificativo negativo) non contano qui.
 */
export function pickSoundEvent(decodes: DecodedMessage[], lastId: number | null, st: RingState, myCall?: string | null, dxCall?: string | null): Picked {
  const newest = decodes.reduce<number | null>((max, d) => (max === null || d.id > max ? d.id : max), null);
  if (lastId === null) return { event: null, sound: null, lastId: newest };
  const fresh = decodes.filter((d) => d.id > lastId && d.id >= 0);
  const now = Date.now();
  const candidates = new Set<SoundEvent>();
  // la chiave e' la stazione che trasmette (il mittente del messaggio): suona solo la prima volta che entra nelle regole
  const keysOf = new Map<SoundEvent, string[]>();
  const newSenders = new Set<string>();
  for (const d of fresh) {
    const sender = (senderOf(d.text)?.call ?? d.text.trim().split(/\s+/)[1] ?? d.text).toUpperCase();
    if (alreadyNotified(sender, now)) continue;
    newSenders.add(sender);
    for (const e of eventsOf(d, myCall, dxCall)) {
      candidates.add(e);
      keysOf.set(e, [...(keysOf.get(e) ?? []), sender]);
    }
  }
  let event: SoundEvent | null = null;
  let sound: string | null = null;
  for (const e of PRIORITY) {
    if (!candidates.has(e)) continue;
    const id = soundFor(e, st);
    if (id) {
      event = e;
      sound = id;
      // da ora queste stazioni non fanno piu' suonare nulla
      for (const key of keysOf.get(e) ?? []) notifiedAt.set(key, now);
      break;
    }
  }
  if (!event && newSenders.size > 0) {
    const id = soundFor('first_message', st);
    if (id) {
      event = 'first_message';
      sound = id;
      for (const key of newSenders) notifiedAt.set(key, now);
    }
  }
  return { event, sound, lastId: newest === null ? lastId : Math.max(lastId, newest) };
}
