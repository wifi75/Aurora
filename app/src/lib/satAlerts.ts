/*
 * Avvisi della pagina Satelliti: preavviso, AOS (il satellite entra nell'area da cui si vede, sopra l'elevazione minima) e LOS.
 * Qui c'e' solo la logica pura: istanti, impostazioni e avvisi gia' emessi in ingresso, avvisi da fare e stato della
 * grafica in uscita. Suono e disegno stanno altrove.
 *
 * Il preavviso e il conto alla rovescia vengono dai passaggi previsti; l'AOS e la LOS veri dal cambio di «visibile»
 * delle posizioni in tempo reale (stesso criterio dei cerchi verdi). Di un passaggio gia' in corso quando si chiede
 * l'elenco l'inizio coincide con l'istante della richiesta e cambia a ogni aggiornamento: per questo un passaggio si
 * riconosce dal satellite e dalla fine (con una tolleranza), mai dall'inizio.
 */

import { DEFAULT_ZOOM_MODE, ZOOM_MODES, type ZoomMode } from './satView';

export type AlertSound = 'bell' | 'radar' | 'arpeggio';
export type SoundChoice = AlertSound | 'off';
export type AlertKind = 'pre' | 'aos' | 'los' | 'rotor';

export interface SatAlertSettings {
  /** Interruttore generale: spento, niente suoni e niente animazioni. */
  enabled: boolean;
  /** 0-100. */
  volume: number;
  /** Minuti di preavviso; 0 = nessun preavviso. */
  leadMin: number;
  /** Solo il satellite scelto, invece di tutti quelli spuntati. */
  onlySelected: boolean;
  soundPre: SoundChoice;
  soundAos: SoundChoice;
  soundLos: SoundChoice;
  /** Minuti di preavviso prima che il rotore si sposti verso il punto di sorgere; 0 = nessuno. Vale solo con il rotore acceso. */
  rotorLeadMin: number;
  soundRotor: SoundChoice;
  /** Zoom automatico della mappa quando un satellite e' in vista (valori vecchi senza il campo: quello di base). */
  zoom: ZoomMode;
}

export const DEFAULT_ALERT_SETTINGS: SatAlertSettings = { enabled: true, volume: 60, leadMin: 5, onlySelected: false, soundPre: 'bell', soundAos: 'radar', soundLos: 'bell', rotorLeadMin: 5, soundRotor: 'arpeggio', zoom: DEFAULT_ZOOM_MODE };

export const ALERT_SOUNDS: readonly AlertSound[] = ['bell', 'radar', 'arpeggio'];
export const ALERT_STORAGE_KEY = 'aurora.sat.alerts';
/** Durata del lampo dopo l'AOS e del dissolvimento dopo la LOS (secondi). */
export const FLASH_S = 6;
export const FADE_S = 4;
/** Oltre questo salto fra due controlli (pausa del browser, orologio che salta) gli eventi non si recuperano. */
export const STALE_S = 60;
/** Due passaggi dello stesso satellite con la fine entro questo scarto sono lo stesso passaggio. */
export const MATCH_S = 90;
const KEEP_S = 3600;

export interface AlertPass {
  norad: number;
  name: string;
  aos_unix: number;
  los_unix: number;
  max_el_deg: number;
  /** Azimut del punto di sorgere (per l'avviso del rotore). */
  aos_az_deg?: number;
}

/** Un avviso gia' emesso: tipo, satellite e fine del passaggio a cui si riferiva. */
export interface AlertRecord {
  kind: AlertKind;
  norad: number;
  los: number;
}

export interface AlertEvent extends AlertRecord {
  name: string;
  /** Solo per il preavviso: secondi all'inizio previsto. */
  secondsToAos?: number;
  /** Solo per l'avviso del rotore: azimut verso cui dovra' spostarsi. */
  az?: number;
}

export interface LiveSat {
  visible: boolean;
  name: string;
}

export interface AlertMark {
  aosAt?: number;
  aosLos?: number;
  losAt?: number;
  losLos?: number;
}

export type AlertPhase = 'pre' | 'flash' | 'live' | 'fade';

export interface AlertView {
  norad: number;
  name: string;
  phase: AlertPhase;
  /** Secondi all'inizio previsto (solo nella fase di preavviso). */
  secondsToAos: number;
}

export interface AlertInput {
  passes: readonly AlertPass[];
  now: number;
  /** L'istante del controllo precedente (`null` al primo). */
  prevNow: number | null;
  settings: SatAlertSettings;
  selected: number | null;
  tracked: readonly number[];
  /** Posizioni attuali e quelle del controllo precedente: norad -> visibile (sopra l'elevazione minima). */
  live: ReadonlyMap<number, LiveSat>;
  prevLive: ReadonlyMap<number, LiveSat>;
  /** Elevazione minima impostata: i passaggi che non la raggiungono non danno avvisi. */
  minEl: number;
  log: readonly AlertRecord[];
  /** Rotore acceso: serve per l'avviso di spostamento. */
  rotorOn?: boolean;
}

const sameSet = (a: AlertRecord, kind: AlertKind, norad: number, los: number) => a.kind === kind && a.norad === norad && Math.abs(a.los - los) <= MATCH_S;

/** Vero se l'avviso (tipo, satellite, fine del passaggio) e' gia' nel registro. */
export function isDone(log: readonly AlertRecord[], kind: AlertKind, norad: number, los: number): boolean {
  return log.some((r) => sameSet(r, kind, norad, los));
}

/** Il satellite e' fra quelli da avvisare con queste impostazioni? */
export function isWatched(norad: number, settings: SatAlertSettings, selected: number | null, tracked: readonly number[]): boolean {
  if (!settings.enabled) return false;
  return settings.onlySelected ? norad === selected : tracked.includes(norad);
}

/** Il passaggio che comprende `now` (con un po' di margine sull'inizio) per quel satellite, se c'e'. */
function coveringPass(passes: readonly AlertPass[], norad: number, now: number, slack = 0): AlertPass | undefined {
  return passes.find((p) => p.norad === norad && p.aos_unix <= now + 120 && p.los_unix >= now - slack);
}

/** Gli avvisi da fare adesso, senza ripetere quelli gia' nel registro e senza recuperare quelli persi in una pausa lunga. */
export function dueAlerts(input: AlertInput): AlertEvent[] {
  const { passes, now, prevNow, settings, selected, tracked, live, prevLive, minEl, log, rotorOn } = input;
  if (!settings.enabled) return [];
  const out: AlertEvent[] = [];
  const seen: AlertRecord[] = [...log];
  const push = (ev: AlertEvent) => {
    if (isDone(seen, ev.kind, ev.norad, ev.los)) return;
    seen.push({ kind: ev.kind, norad: ev.norad, los: ev.los });
    out.push(ev);
  };

  // preavviso: dai passaggi previsti, mentre manca meno del preavviso e il passaggio non e' ancora iniziato
  if (settings.leadMin > 0) {
    for (const p of passes) {
      if (!isWatched(p.norad, settings, selected, tracked)) continue;
      if (p.max_el_deg < minEl || p.aos_unix <= now) continue;
      if (now < p.aos_unix - settings.leadMin * 60) continue;
      push({ kind: 'pre', norad: p.norad, name: p.name, los: p.los_unix, secondsToAos: p.aos_unix - now });
    }
  }

  // preavviso del rotore: prima che si sposti verso il punto di sorgere
  if (rotorOn && settings.rotorLeadMin > 0) {
    for (const p of passes) {
      if (!isWatched(p.norad, settings, selected, tracked)) continue;
      if (p.max_el_deg < minEl || p.aos_unix <= now) continue;
      if (now < p.aos_unix - settings.rotorLeadMin * 60) continue;
      push({ kind: 'rotor', norad: p.norad, name: p.name, los: p.los_unix, secondsToAos: p.aos_unix - now, az: p.aos_az_deg });
    }
  }

  // AOS e LOS: dal cambio di «visibile» fra due controlli ravvicinati
  const fresh = prevNow !== null && now >= prevNow && now - prevNow <= STALE_S;
  if (fresh) {
    for (const [norad, cur] of live) {
      if (!isWatched(norad, settings, selected, tracked)) continue;
      const before = prevLive.get(norad);
      if (!before || before.visible === cur.visible) continue;
      const pass = coveringPass(passes, norad, now, 120);
      const name = pass?.name ?? cur.name;
      const los = pass?.los_unix ?? Math.round(now);
      push({ kind: cur.visible ? 'aos' : 'los', norad, name, los });
    }
  }
  return out;
}

/** La scritta del rotore: il prossimo passaggio verso cui il rotore sta per spostarsi, dal momento del preavviso fino all'AOS. */
export interface RotorNotice {
  norad: number;
  name: string;
  az: number | null;
  secondsToAos: number;
}

export function rotorNotice(input: Pick<AlertInput, 'passes' | 'now' | 'settings' | 'selected' | 'tracked' | 'minEl' | 'rotorOn'>): RotorNotice | null {
  const { passes, now, settings, selected, tracked, minEl, rotorOn } = input;
  if (!rotorOn || !settings.enabled || settings.rotorLeadMin <= 0) return null;
  let best: AlertPass | null = null;
  for (const p of passes) {
    if (!isWatched(p.norad, settings, selected, tracked)) continue;
    if (p.max_el_deg < minEl || p.aos_unix <= now) continue;
    if (now < p.aos_unix - settings.rotorLeadMin * 60) continue;
    if (!best || p.aos_unix < best.aos_unix) best = p;
  }
  return best ? { norad: best.norad, name: best.name, az: best.aos_az_deg ?? null, secondsToAos: Math.max(0, best.aos_unix - now) } : null;
}

/** Aggiunge gli avvisi fatti al registro e toglie quelli di passaggi finiti da piu' di un'ora. */
export function remember(log: readonly AlertRecord[], events: readonly AlertEvent[], now: number): AlertRecord[] {
  const next = log.filter((r) => r.los > now - KEEP_S);
  for (const ev of events) next.push({ kind: ev.kind, norad: ev.norad, los: ev.los });
  return next;
}

/** Aggiorna i segni di AOS e LOS (per lampo e dissolvimento) con gli avvisi appena fatti. */
export function applyMarks(marks: ReadonlyMap<number, AlertMark>, events: readonly AlertEvent[], now: number): Map<number, AlertMark> {
  const next = new Map<number, AlertMark>();
  for (const [norad, m] of marks) {
    if ((m.losAt ?? m.aosAt ?? 0) > now - KEEP_S) next.set(norad, m);
  }
  for (const ev of events) {
    if (ev.kind === 'aos') next.set(ev.norad, { ...next.get(ev.norad), aosAt: now, aosLos: ev.los });
    if (ev.kind === 'los') next.set(ev.norad, { ...next.get(ev.norad), losAt: now, losLos: ev.los });
  }
  return next;
}

/** Cosa mostrare adesso sulla mappa: preavviso, lampo, anello fisso o dissolvimento, per ogni satellite da avvisare. */
export function alertViews(input: Omit<AlertInput, 'prevNow' | 'prevLive'> & { marks: ReadonlyMap<number, AlertMark> }): AlertView[] {
  const { passes, now, settings, selected, tracked, live, marks, minEl, log } = input;
  if (!settings.enabled) return [];
  const out: AlertView[] = [];
  const norads = new Set<number>([...live.keys(), ...passes.map((p) => p.norad)]);
  for (const norad of norads) {
    if (!isWatched(norad, settings, selected, tracked)) continue;
    const l = live.get(norad);
    const mark = marks.get(norad);
    const name = l?.name ?? passes.find((p) => p.norad === norad)?.name ?? String(norad);
    if (l?.visible) {
      const flash = mark?.aosAt !== undefined && now - mark.aosAt >= 0 && now - mark.aosAt < FLASH_S;
      out.push({ norad, name, phase: flash ? 'flash' : 'live', secondsToAos: 0 });
      continue;
    }
    if (mark?.losAt !== undefined && now - mark.losAt >= 0 && now - mark.losAt < FADE_S) {
      out.push({ norad, name, phase: 'fade', secondsToAos: 0 });
      continue;
    }
    if (settings.leadMin <= 0) continue;
    for (const p of passes) {
      if (p.norad !== norad || p.max_el_deg < minEl || now >= p.los_unix) continue;
      if (now < p.aos_unix - settings.leadMin * 60) continue;
      // passaggio gia' visto da vicino: dopo l'inizio resta acceso solo se il preavviso era stato dato e l'AOS non e' ancora scattato
      if (p.aos_unix <= now && !isDone(log, 'pre', norad, p.los_unix)) continue;
      if (mark?.aosLos !== undefined && Math.abs(mark.aosLos - p.los_unix) <= MATCH_S) continue;
      if (mark?.losLos !== undefined && Math.abs(mark.losLos - p.los_unix) <= MATCH_S) continue;
      out.push({ norad, name, phase: 'pre', secondsToAos: Math.max(0, p.aos_unix - now) });
      break;
    }
  }
  return out;
}

/** Unisce elenchi di passaggi togliendo i doppioni (stesso satellite, fine entro la tolleranza); il primo elenco vince. */
export function mergePasses(...lists: (readonly AlertPass[])[]): AlertPass[] {
  const out: AlertPass[] = [];
  for (const list of lists) {
    for (const p of list) {
      if (!out.some((q) => q.norad === p.norad && Math.abs(q.los_unix - p.los_unix) <= MATCH_S)) out.push(p);
    }
  }
  return out;
}

const clamp = (v: unknown, lo: number, hi: number, dflt: number): number => (typeof v === 'number' && Number.isFinite(v) ? Math.max(lo, Math.min(hi, Math.round(v))) : dflt);
const choice = (v: unknown, dflt: SoundChoice): SoundChoice => (v === 'off' || (typeof v === 'string' && (ALERT_SOUNDS as readonly string[]).includes(v)) ? (v as SoundChoice) : dflt);

/** Impostazioni lette da un valore qualunque (memoria del browser): quello che non torna riprende il valore di base. */
export function sanitizeAlertSettings(raw: unknown): SatAlertSettings {
  const d = DEFAULT_ALERT_SETTINGS;
  if (!raw || typeof raw !== 'object') return { ...d };
  const r = raw as Record<string, unknown>;
  return {
    enabled: typeof r.enabled === 'boolean' ? r.enabled : d.enabled,
    volume: clamp(r.volume, 0, 100, d.volume),
    leadMin: clamp(r.leadMin, 0, 30, d.leadMin),
    onlySelected: typeof r.onlySelected === 'boolean' ? r.onlySelected : d.onlySelected,
    soundPre: choice(r.soundPre, d.soundPre),
    soundAos: choice(r.soundAos, d.soundAos),
    soundLos: choice(r.soundLos, d.soundLos),
    rotorLeadMin: clamp(r.rotorLeadMin, 0, 30, d.rotorLeadMin),
    soundRotor: choice(r.soundRotor, d.soundRotor),
    zoom: typeof r.zoom === 'string' && (ZOOM_MODES as readonly string[]).includes(r.zoom) ? (r.zoom as ZoomMode) : d.zoom,
  };
}

type Store = Pick<Storage, 'getItem' | 'setItem'>;

function defaultStore(): Store | null {
  try {
    return typeof window === 'undefined' ? null : window.localStorage;
  } catch {
    return null;
  }
}

export function loadAlertSettings(store: Store | null = defaultStore()): SatAlertSettings {
  try {
    const text = store?.getItem(ALERT_STORAGE_KEY);
    return sanitizeAlertSettings(text ? JSON.parse(text) : null);
  } catch {
    return { ...DEFAULT_ALERT_SETTINGS };
  }
}

export function saveAlertSettings(settings: SatAlertSettings, store: Store | null = defaultStore()): void {
  try {
    store?.setItem(ALERT_STORAGE_KEY, JSON.stringify(settings));
  } catch {
    /* memoria del browser non disponibile: le scelte valgono fino alla chiusura della pagina */
  }
}
