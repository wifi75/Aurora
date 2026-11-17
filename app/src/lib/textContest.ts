/**
 * Il contest dei modi testuali scelto dal menu Contest: finestra di date dei doppi, categorie valide, doppi senza banda o modo, macro
 * del contest unite a quelle del modo, numero da mandare e cattura dello scambio ricevuto (numero o dominio) dal testo.
 */
import type { Domain, TextContest } from '../api/contestTesto';
import { buildLogIndex, contestStatus, type LogStatus } from './logStatus';
import { qsoTimeMs } from './logWindow';
import type { Macro } from './macroLang';

/** Una data `aaaammgghhmmss` (UTC) in ms, o `null`. */
export function contestDateMs(d: string): number | null {
  const m = /^(\d{4})(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})$/.exec(d.trim());
  if (!m) return null;
  const ms = Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3]), Number(m[4]), Number(m[5]), Number(m[6]));
  return Number.isFinite(ms) ? ms : null;
}

const withYear = (ms: number, years: number) => {
  const d = new Date(ms);
  return Date.UTC(d.getUTCFullYear() + years, d.getUTCMonth(), d.getUTCDate(), d.getUTCHours(), d.getUTCMinutes(), d.getUTCSeconds());
};

/**
 * La finestra del contest per l'anno di adesso: le date del file valgono ogni anno (conta il giorno e l'ora), quindi si portano
 * nell'anno corrente mantenendo la durata. Senza date: `null` (contano tutti i QSO).
 */
export function contestWindow(c: Pick<TextContest, 'start' | 'end'>, nowMs: number): { from: number | null; to: number | null } | null {
  const a = contestDateMs(c.start);
  const b = contestDateMs(c.end);
  if (a == null && b == null) return null;
  const ref = a ?? b ?? nowMs;
  const shift = new Date(nowMs).getUTCFullYear() - new Date(ref).getUTCFullYear();
  return { from: a == null ? null : withYear(a, shift), to: b == null ? null : withYear(b, shift) };
}

export type ModeCategory = 'cw' | 'phone' | 'digital';

/** La categoria di un modo: CW, fonia (SSB, AM, FM) o digitale (tutto il resto). */
export function modeCategory(mode: string): ModeCategory {
  const m = mode.trim().toUpperCase();
  if (m === 'CW') return 'cw';
  if (['SSB', 'USB', 'LSB', 'AM', 'FM', 'PHONE'].includes(m)) return 'phone';
  return 'digital';
}

/** Il modo conta per il contest? Nessuna categoria scelta = tutte. */
export function categoryValid(c: Pick<TextContest, 'valid_cw' | 'valid_phone' | 'valid_digital'>, mode: string): boolean {
  if (!c.valid_cw && !c.valid_phone && !c.valid_digital) return true;
  const k = modeCategory(mode);
  return k === 'cw' ? c.valid_cw : k === 'phone' ? c.valid_phone : c.valid_digital;
}

type Q = { call: string; band: string; mode: string; date: string; time: string };

/** I QSO del log che contano per il contest: dentro la finestra e di una categoria valida. */
export function contestQsos<T extends Q>(qsos: readonly T[], c: TextContest, nowMs: number): T[] {
  const w = contestWindow(c, nowMs);
  return qsos.filter((q) => {
    if (!categoryValid(c, q.mode)) return false;
    if (!w) return true;
    const t = qsoTimeMs(q);
    if (!Number.isFinite(t)) return false;
    return !(w.from != null && t < w.from) && !(w.to != null && t > w.to);
  });
}

/**
 * Il segno di BIGScan con un contest dei modi testuali: doppio se lo stesso nominativo e' gia' nei QSO del contest con la stessa banda
 * (salvo «doppio su ogni banda») e lo stesso modo (salvo «doppio in ogni modo»); altrimenti le novita' rispetto ai QSO del contest.
 */
export function textContestStatus(qsos: readonly Q[], c: TextContest, call: string, band: string | null, mode: string): LogStatus {
  const key = call.trim().toUpperCase();
  if (!categoryValid(c, mode)) return null;
  const dupe = qsos.some((q) => q.call.trim().toUpperCase() === key && (c.dupe_no_band || !band || q.band.toLowerCase() === band.toLowerCase()) && (c.dupe_no_mode || q.mode.toUpperCase() === mode.toUpperCase()));
  if (dupe) return 'dupe';
  return contestStatus(null, buildLogIndex(qsos.map((q) => ({ ...q, country: '' }))), call, band, mode);
}

/** Le macro in uso con il contest: quelle del contest sostituiscono le altre sugli stessi tasti; senza «Unisci» restano solo loro. */
export function withContestMacros(effective: ReadonlyMap<string, Macro>, contest: readonly Macro[] | undefined, merge: boolean): Map<string, Macro> {
  if (!contest || contest.length === 0) return new Map(effective);
  const out = merge ? new Map(effective) : new Map<string, Macro>();
  for (const m of contest) out.set(m.name, m);
  return out;
}

/** Il primo numero da mandare: le cifre di «Numero da mandare» (almeno 1). */
export function startSerial(nrs: string): number {
  const n = Number((nrs.match(/\d+/) ?? [''])[0]);
  return Number.isInteger(n) && n >= 1 ? Math.min(n, 99999) : 1;
}

/** Il dominio di una parola: prima nelle sezioni il cui prefisso inizia il nominativo, poi in tutto l'elenco. */
export function domainLookup(d: Domain | null | undefined, word: string, call: string): string | null {
  if (!d) return null;
  const w = word.trim().toUpperCase();
  const c = call.trim().toUpperCase();
  if (!w) return null;
  const inSection = d.entries.find((e) => e.key === w && e.section !== '' && c.startsWith(e.section));
  return (inSection ?? d.entries.find((e) => e.key === w))?.value ?? null;
}

const RST = /^(5[1-9][1-9]|5[1-9]|5NN)$/i;

/** Il testo senza le righe di quello che abbiamo trasmesso noi (l'eco «» …» della trasmissione e il testo mandato). */
export function withoutOwnEcho(text: string): string {
  return text
    .split('\n')
    .filter((l) => !l.startsWith('»'))
    .join('\n');
}

/**
 * Lo scambio ricevuto preso dal testo: le parole dopo l'ultima volta che compare il nominativo del QSO. Prima il dominio (se l'elenco
 * c'e'), poi il numero (il rapporto 599/59 davanti si salta). `null` se non c'e' niente da prendere. Il nostro testo trasmesso (righe
 * «» …»: il testo mandato e l'eco della trasmissione) non conta: con l'eco accesa il nominativo del corrispondente seguito dal nostro
 * numero e' proprio quello che trasmettiamo. Il numero uguale al nostro non si scarta (all'inizio tutti mandano 001).
 */
export function grabExchange(text: string, call: string, opts: { number: boolean; domain: Domain | null | undefined }): string | null {
  const c = call.trim().toUpperCase();
  if (c.length < 3) return null;
  const words = withoutOwnEcho(text).toUpperCase().split(/[^A-Z0-9/]+/).filter(Boolean);
  let at = -1;
  for (let i = words.length - 1; i >= 0; i--) if (words[i] === c) { at = i; break; }
  if (at < 0) return null;
  const after = words.slice(at + 1, at + 9).filter((w) => w !== 'DE' && w !== 'TU' && w !== 'UR' && w !== 'NR');
  if (opts.domain) for (const w of after) {
    if (RST.test(w)) continue;
    const v = domainLookup(opts.domain, w, c);
    if (v) return v;
  }
  if (opts.number) {
    const nums = after.filter((w) => /^\d{1,5}$/.test(w));
    const n = nums.find((w) => !RST.test(w)) ?? null;
    return n;
  }
  return null;
}

// ---------------------------------------------------------------- finestra del contest: controllo, stato e proposta dell'anno

const pad2 = (n: number) => String(n).padStart(2, '0');

/** Una data `aaaammgghhmmss` esiste davvero (niente 31 novembre o ora 25). */
export function validContestDate(d: string): boolean {
  const m = /^(\d{4})(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})$/.exec(d.trim());
  if (!m) return false;
  const [y = 0, mo = 0, da = 0, h = 0, mi = 0, s = 0] = m.slice(1).map(Number);
  const x = new Date(Date.UTC(y, mo - 1, da, h, mi, s));
  return x.getUTCFullYear() === y && x.getUTCMonth() === mo - 1 && x.getUTCDate() === da && x.getUTCHours() === h && x.getUTCMinutes() === mi && x.getUTCSeconds() === s;
}

/** Controlla inizio e fine scritti nel dialogo: ognuno vuoto o valido; con tutti e due, la fine dopo l'inizio. */
export function checkContestDates(start: string, end: string): 'ok' | 'date' | 'order' {
  const s = start.trim();
  const e = end.trim();
  if ((s !== '' && !validContestDate(s)) || (e !== '' && !validContestDate(e))) return 'date';
  if (s !== '' && e !== '') {
    const a = contestDateMs(s);
    const b = contestDateMs(e);
    if (a == null || b == null || b <= a) return 'order';
  }
  return 'ok';
}

const fmtContestDate = (ms: number) => {
  const d = new Date(ms);
  return `${d.getUTCFullYear()}${pad2(d.getUTCMonth() + 1)}${pad2(d.getUTCDate())}${pad2(d.getUTCHours())}${pad2(d.getUTCMinutes())}${pad2(d.getUTCSeconds())}`;
};

/**
 * L'ultimo fine settimana completo del mese (`month` 0-11): dal sabato 00:00:00 alla domenica 23:59:59 UTC, con la domenica ancora
 * dentro il mese (ottobre 2026: 24 e 25, non il 31 con la domenica a novembre).
 */
export function lastFullWeekend(year: number, month: number): { from: number; to: number } {
  const lastDay = new Date(Date.UTC(year, month + 1, 0));
  const sunday = lastDay.getUTCDate() - lastDay.getUTCDay();
  return { from: Date.UTC(year, month, sunday - 1, 0, 0, 0), to: Date.UTC(year, month, sunday, 23, 59, 59) };
}

/**
 * L'n-esimo fine settimana completo del mese (`month` 0-11, `n` da 1): i sabati con la domenica ancora nel mese; il sabato 1 con la
 * domenica il 2 e' gia' il primo. Dalle `fromHour` del sabato alle `toHour` (`:59:59` se 23) della domenica, UTC.
 */
export function nthFullWeekend(year: number, month: number, n: number, fromHour = 0, toHour = 23): { from: number; to: number } {
  const days = new Date(Date.UTC(year, month + 1, 0)).getUTCDate();
  const saturdays: number[] = [];
  for (let d = 1; d < days; d++) if (new Date(Date.UTC(year, month, d)).getUTCDay() === 6) saturdays.push(d);
  const sat = saturdays[Math.min(n, saturdays.length) - 1] ?? 1;
  const end = toHour >= 23 ? Date.UTC(year, month, sat + 1, 23, 59, 59) : Date.UTC(year, month, sat + 1, toHour, 0, 0);
  return { from: Date.UTC(year, month, sat, fromHour, 0, 0), to: end };
}

export interface WindowProposal {
  /** Edizione (`SSB`, `CW`) o vuoto se il contest ne ha una sola. */
  tag: string;
  start: string;
  end: string;
}

/**
 * Le finestre proposte per l'anno `year` ai contest ricorrenti con regola semplice, riconosciuti dal nome (CQ WW DX, CQ WW WPX, Japan International DX):
 * ultimo fine settimana completo di ottobre (SSB) o novembre (CW) per il CQ WW DX, di marzo (SSB) o maggio (CW) per il WPX, secondo
 * fine settimana completo di aprile (CW) o novembre (fonia) per il Japan International DX.
 * Il nome senza modo (`CQ WW DX Contest`, che raccoglie CW e SSB) da' le due edizioni. Un contest che non si riconosce non ha proposte:
 * l'utente imposta le date a mano. Le date dei file distribuiti non servono: sono di anni passati e in parte invertite fra CW e SSB.
 */
export function windowProposals(name: string, year: number): WindowProposal[] {
  const n = name.toUpperCase().replace(/[^A-Z0-9]+/g, ' ').trim();
  const make = (tag: string, month: number): WindowProposal => {
    const w = lastFullWeekend(year, month);
    return { tag, start: fmtContestDate(w.from), end: fmtContestDate(w.to) };
  };
  const dx = /^CQ WW DX(?: (SSB|CW))?(?: CONTEST)?$/.exec(n);
  if (dx) return dx[1] === 'SSB' ? [make('SSB', 9)] : dx[1] === 'CW' ? [make('CW', 10)] : [make('SSB', 9), make('CW', 10)];
  // Japan International DX: CW il 2o fine settimana completo di aprile, fonia di novembre, dal sabato 07:00 alla domenica 13:00 UTC
  const jidx = /^JAPAN INTERNATIONAL DX (CW|SSB|PHONE)(?: CONTEST)?$/.exec(n);
  if (jidx) {
    const w = jidx[1] === 'CW' ? nthFullWeekend(year, 3, 2, 7, 13) : nthFullWeekend(year, 10, 2, 7, 13);
    return [{ tag: jidx[1] === 'CW' ? 'CW' : 'SSB', start: fmtContestDate(w.from), end: fmtContestDate(w.to) }];
  }
  const wpx = /^CQ (?:WW )?WPX (SSB|CW)(?: CONTEST)?$/.exec(n);
  if (wpx) return [wpx[1] === 'SSB' ? make('SSB', 2) : make('CW', 4)];
  return [];
}

export type WindowStatus = 'none' | 'before' | 'active' | 'expired';

/** Dove e' adesso rispetto alla finestra del contest: senza date `none`, altrimenti prima, dentro o dopo. */
export function contestWindowStatus(c: Pick<TextContest, 'start' | 'end'>, nowMs: number): { status: WindowStatus; window: { from: number | null; to: number | null } | null } {
  const w = contestWindow(c, nowMs);
  if (!w) return { status: 'none', window: null };
  if (w.from != null && nowMs < w.from) return { status: 'before', window: w };
  if (w.to != null && nowMs > w.to) return { status: 'expired', window: w };
  return { status: 'active', window: w };
}

/** Quale avviso mostrare quando la finestra del contest non vale adesso: non ancora iniziato (con l'inizio in UTC), scaduta, non impostata. */
export function windowNoticeInfo(c: Pick<TextContest, 'start' | 'end'>, nowMs: number): { key: 'ssbc.windowBefore' | 'ssbc.windowExpired' | 'ssbc.windowOff'; when: string } {
  const { status, window } = contestWindowStatus(c, nowMs);
  if (status === 'before' && window?.from != null) return { key: 'ssbc.windowBefore', when: new Date(window.from).toISOString().slice(0, 16).replace('T', ' ') };
  if (status === 'expired') return { key: 'ssbc.windowExpired', when: '' };
  return { key: 'ssbc.windowOff', when: '' };
}

/**
 * Le date del file non sono dell'anno in corso (o mancano): per un contest con una sola edizione conosciuta si propongono nel
 * dialogo. `null` se le date sono gia' di quest'anno, se il nome non e' riconosciuto o se ha piu' edizioni (si sceglie a mano).
 */
export function staleWindowProposal(c: Pick<TextContest, 'name' | 'start' | 'end'>, nowMs: number): WindowProposal | null {
  const year = new Date(nowMs).getUTCFullYear();
  const a = contestDateMs(c.start);
  if (a != null && new Date(a).getUTCFullYear() === year) return null;
  const p = windowProposals(c.name, year);
  return p.length === 1 ? (p[0] ?? null) : null;
}
