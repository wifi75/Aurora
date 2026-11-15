/**
 * I comandi delle macro oltre a quelli di base (`macroLang.ts`), famiglia per famiglia: i valori di testo (`extraText`), i comandi che
 * cambiano i dati della macro mentre si legge (`special`) e quelli che la pagina esegue (`extraAction`, quasi tutti `cmd`, vedi
 * `macroRuntime.ts`).
 */
import { bearingDeg, distanceKm } from './geo';
import { parseIgnoreBefore } from './logWindow';
import { gridToLatLon } from './maidenhead';
import type { MacroAction, MacroContext } from './macroLang';

/** Il nominativo senza i suffissi (/P, /M...) e senza il prefisso di un altro paese. */
export function mainCall(call: string): string {
  const parts = call.toUpperCase().split('/').filter(Boolean);
  if (parts.length <= 1) return parts[0] ?? '';
  // si tiene la parte piu' lunga e con una cifra: quella e' il nominativo vero
  return [...parts].sort((a, b) => Number(/\d/.test(b)) - Number(/\d/.test(a)) || b.length - a.length)[0] ?? '';
}

/** Lo stato di una macro mentre si espande: i dati (si possono cambiare) e dove finiscono testo e comandi. */
export interface MacroRun {
  ctx: MacroContext;
  text: (s: string) => void;
  act: (a: MacroAction) => void;
  /** Espande un altro pezzo di script un livello piu' in basso (macro dentro una macro). */
  expand: (script: string) => void;
  setHidden: (hidden: boolean) => void;
}

const pad = (n: number, w = 2) => String(n).padStart(w, '0');
const cmd = (tag: string, arg?: string): MacroAction => ({ type: 'cmd', tag, arg: (arg ?? '').trim() });

/** OM o YL dal nome: i nomi femminili piu' comuni (finiscono in A, salvo i maschili noti). */
const MALE_A = new Set(['ANDREA', 'LUCA', 'NICOLA', 'MATTIA', 'ELIA', 'ENEA', 'JOSHUA', 'SASHA', 'MISHA', 'KOSTA', 'ILJA', 'ILYA', 'MIKA', 'JONA', 'BORJA', 'GARCIA', 'COSTA', 'BATTISTA', 'TOSHIA']);
export function omOrYl(name: string): 'OM' | 'YL' {
  const n = name.trim().split(/\s+/)[0]?.toUpperCase() ?? '';
  return n.length > 2 && n.endsWith('A') && !MALE_A.has(n) ? 'YL' : 'OM';
}

/** Distanza (km) e direzione dal proprio locatore a quello del corrispondente; `null` se manca uno dei due. */
function path(c: MacroContext): { km: number; az: number } | null {
  const from = gridToLatLon(c.mywwloc);
  const to = gridToLatLon(c.wwloc);
  return from && to ? { km: distanceKm(from, to), az: bearingDeg(from, to) } : null;
}

/** I numeri in CW abbreviati (T = 0, O = 0, A = 1, N = 9) tornano cifre. */
export function fromCutNumbers(s: string): string {
  return /\d/.test(s) || /^[TOAN]+$/i.test(s.trim()) ? s.toUpperCase().replace(/[TO]/g, '0').replace(/A/g, '1').replace(/N/g, '9') : s;
}

/** L'istante del QSO gia' fatto con questo nominativo: dal log del contest se c'e', se no dal log (stessa banda e modo). */
export function beforeMs(c: MacroContext): number | null {
  return c.contestPrev?.beforeMs ?? c.before?.ms ?? null;
}
const hhmm = (ms: number) => {
  const d = new Date(ms);
  return `${pad(d.getUTCHours())}${pad(d.getUTCMinutes())}`;
};
const ymd = (ms: number) => new Date(ms).toISOString().slice(0, 10);
const hm = (ms: number) => new Date(ms).toISOString().slice(11, 16);

function family1(tag: string, arg: string | undefined, c: MacroContext): string | null {
  const p = c.prevQso;
  switch (tag) {
    case 'ACALL': return mainCall(c.call);
    case 'CALLPOST': return c.call.trim() !== '' && arg !== undefined ? `${c.call.toUpperCase()}${arg}` : '';
    case 'POST': {
      if (arg === undefined) return null;
      const [yes = '', no = ''] = arg.split(',');
      return c.call.trim() !== '' ? yes : no;
    }
    case 'OM': return omOrYl(c.name);
    case 'DIST': return path(c) ? String(path(c)?.km) : '?';
    case 'MILES': return path(c) ? String(Math.round((path(c)?.km ?? 0) / 1.609344)) : '?';
    case 'AZ': return path(c) ? String(path(c)?.az) : '?';
    case 'PREVNRR': return p?.nrr ?? '';
    case 'PREVCCALL': return mainCall(p?.call ?? '');
    case 'PREVNAME': return p?.name ?? '';
    case 'PREVQTH': return p?.qth ?? '';
    case 'PREVRSTR': return p?.rstr ?? '';
    case 'PREVCTIME': return p?.ms != null ? hhmm(p.ms) : '';
    case 'PREVSUF2': return (p?.call ?? '').replace(/\/.*$/, '').slice(-2);
    case 'NRRPOST': return c.nrr.trim() !== '' ? `${fromCutNumbers(c.nrr.trim())}${arg ?? ''}` : '';
    case 'QSOBEFORE': {
      const ms = beforeMs(c);
      return ms != null ? `${ymd(ms)} ${hm(ms)}` : '';
    }
    case 'TIME1':
    case 'QSOBEFORETIME1': {
      const ms = beforeMs(c);
      return ms != null ? hhmm(ms) : '';
    }
    case 'QSOBEFOREMODE': return c.before?.mode ?? '';
    case 'QSOBEFOREBAND': return c.before?.band ?? '';
    default: return null;
  }
}

function special1(tag: string, arg: string | undefined, r: MacroRun): boolean {
  const c = r.ctx;
  if (tag === 'ONQSOBEFORE') {
    // con il nominativo gia' lavorato si esegue la macro indicata
    const text = c.macros?.get((arg ?? '').trim().toUpperCase());
    if (beforeMs(c) != null && text !== undefined) r.expand(text);
    return true;
  }
  if (tag === 'LASTCALL') {
    const call = (c.prevQso?.call ?? c.contestPrev?.lastCall ?? '').toUpperCase();
    if (call !== '') {
      c.call = call;
      r.act({ type: 'setField', field: 'call', value: call });
    }
    return true;
  }
  return false;
}

type Field = 'call' | 'name' | 'qth' | 'rsts' | 'rstr' | 'nrs' | 'nrr' | 'notes';
const SEL_FIELDS: Record<string, Field> = { SELCALL: 'call', SELNAME: 'name', SELQTH: 'qth', SELRSTS: 'rsts', SELRSTR: 'rstr', SELNRS: 'nrs', SELNRR: 'nrr' };

/** Mette un valore in un campo del QSO: vale subito per il resto della macro e la pagina lo scrive nel log. */
export function setField(r: MacroRun, field: Field, value: string): void {
  const v = field === 'call' ? value.toUpperCase() : value;
  r.ctx[field] = v;
  r.act({ type: 'setField', field, value: v });
}

function special2(tag: string, r: MacroRun): boolean {
  const field = SEL_FIELDS[tag];
  if (field) {
    const sel = (r.ctx.selection ?? '').trim();
    // il nominativo e i rapporti sono una parola sola; nome e QTH tutta la selezione
    const value = field === 'name' || field === 'qth' ? sel.replace(/\s+/g, ' ') : (sel.split(/\s+/)[0] ?? '');
    if (value !== '') setField(r, field, value);
    return true;
  }
  if (tag === 'SELRXTEXT') {
    r.text((r.ctx.selection ?? '').trim());
    return true;
  }
  return false;
}

export const PHRASE_LANGS = ['it', 'en', 'de', 'fr', 'es', 'pt', 'nl'] as const;
export type PhraseLang = (typeof PHRASE_LANGS)[number];
/** Le frasi automatiche nelle lingue (solo lettere senza accenti: passano anche in RTTY). */
const PHRASES: Record<string, readonly [string, string, string, string, string, string, string]> = {
  INTDE: ['da', 'de', 'von', 'de', 'de', 'de', 'van'],
  INTQSL: ['QSL, grazie', 'QSL, thanks', 'QSL, danke', 'QSL, merci', 'QSL, gracias', 'QSL, obrigado', 'QSL, bedankt'],
  INTAGNNR: ['ripeti il numero, per favore', 'please repeat your number', 'bitte die Nummer wiederholen', 'repetez le numero SVP', 'repita el numero, por favor', 'repita o numero, por favor', 'herhaal het nummer a.u.b.'],
  INTAGNCALL: ['ripeti il nominativo, per favore', 'please repeat your call', 'bitte das Rufzeichen wiederholen', 'repetez votre indicatif SVP', 'repita su indicativo, por favor', 'repita o indicativo, por favor', 'herhaal je roepnaam a.u.b.'],
  INTQRZ: ['chi chiama?', 'who is calling?', 'wer ruft?', 'qui appelle?', 'quien llama?', 'quem chama?', 'wie roept er?'],
  INTQB4: ['ci siamo gia collegati', 'we worked before', 'wir hatten schon ein QSO', 'nous avons deja fait QSO', 'ya hemos hecho QSO', 'ja fizemos QSO', 'we hebben al eerder een QSO gehad'],
  INTQRL: ['la frequenza e libera?', 'is this frequency in use?', 'ist die Frequenz frei?', 'la frequence est-elle libre?', 'esta libre la frecuencia?', 'a frequencia esta livre?', 'is deze frequentie vrij?'],
  INTCQ: ['chiamata generale', 'calling CQ', 'allgemeiner Anruf', 'appel general', 'llamada general', 'chamada geral', 'algemene oproep'],
  INTE: ['e', 'and', 'und', 'et', 'y', 'e', 'en'],
};
const INT_GA: Record<PhraseLang, readonly [string, string, string]> = {
  it: ['Buongiorno', 'Buon pomeriggio', 'Buonasera'],
  en: ['Good morning', 'Good afternoon', 'Good evening'],
  de: ['Guten Morgen', 'Guten Tag', 'Guten Abend'],
  fr: ['Bonjour', 'Bon apres-midi', 'Bonsoir'],
  es: ['Buenos dias', 'Buenas tardes', 'Buenas noches'],
  pt: ['Bom dia', 'Boa tarde', 'Boa noite'],
  nl: ['Goedemorgen', 'Goedemiddag', 'Goedenavond'],
};

/** La lingua del corrispondente dal prefisso del nominativo (inglese se non si sa). */
export function langOfCall(call: string): PhraseLang {
  const c = mainCall(call);
  if (/^I[A-Z0-9]/.test(c) || /^HV|^T7|^9H/.test(c)) return 'it';
  if (/^(D[A-R]|OE|HB0|DL)/.test(c) || /^HB9/.test(c)) return 'de';
  if (/^(F|TM|TK|FY|FM|FG|FR|ON|HB9)/.test(c)) return /^ON/.test(c) ? 'nl' : 'fr';
  if (/^(P[A-I])/.test(c)) return 'nl';
  if (/^(E[A-H]|LU|L[O-W]|XE|CE|CX|HK|YV|OA|CO|CM|TI|HP|HC|CP|ZP|YS|YN|HR|HI|TG)/.test(c)) return 'es';
  if (/^(P[P-Y]|ZV|ZW|ZX|ZY|ZZ|CT|CS|CR|CU|D2|D4|C9|S9|J5)/.test(c)) return 'pt';
  return 'en';
}

/** La lingua delle frasi: quella imposta con FORCELANGUAGE o LANG_, se no quella del nominativo. */
export function phraseLang(c: MacroContext): PhraseLang {
  const forced = (c.lang ?? '').toLowerCase();
  return (PHRASE_LANGS as readonly string[]).includes(forced) ? (forced as PhraseLang) : langOfCall(c.call);
}

/** Il momento della giornata (ora locale): 0 mattino, 1 pomeriggio, 2 sera, 3 notte. */
export function daySlot(now: Date): 0 | 1 | 2 | 3 {
  const h = now.getHours();
  return h >= 4 && h < 12 ? 0 : h < 18 && h >= 12 ? 1 : h >= 18 && h < 22 ? 2 : 3;
}

function family3(tag: string, c: MacroContext): string | null {
  const lang = phraseLang(c);
  const row = PHRASES[tag];
  if (row) return row[PHRASE_LANGS.indexOf(lang)] ?? row[1];
  if (tag === 'INTGA') return INT_GA[lang][Math.min(2, daySlot(c.now))] ?? '';
  return null;
}

function special3(tag: string, arg: string | undefined, r: MacroRun): boolean {
  const c = r.ctx;
  if (tag === 'GM' || tag === 'GE' || tag === 'GN') {
    // il saluto del momento; se l'utente ha la macro gm, ga, ge o gn di quel momento si usa quella
    const slot = daySlot(c.now);
    const name = (['GM', 'GA', 'GE', 'GN'] as const)[slot];
    const user = c.macros?.get(name);
    if (user !== undefined && name !== tag) r.expand(user);
    else r.text(name);
    return true;
  }
  const lang = /^LANG_([A-Z]{2})$/.exec(tag)?.[1] ?? (tag === 'FORCELANGUAGE' || tag === 'LANGUAGE' ? (arg ?? '').trim().slice(0, 2).toUpperCase() : null);
  if (lang !== null) {
    c.lang = lang.toLowerCase();
    if (tag !== 'LANGUAGE' || lang !== '') r.act(cmd('FORCELANGUAGE', lang.toLowerCase()));
    return true;
  }
  return false;
}

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
/** Data e ora con un formato alla strftime: %H %M %S %d %m %Y %y %j %b %A(nno breve). Senza `%` restano le lettere scritte. */
export function formatTime(d: Date, fmt: string, utc: boolean): string {
  const g = utc
    ? { y: d.getUTCFullYear(), mo: d.getUTCMonth(), da: d.getUTCDate(), h: d.getUTCHours(), mi: d.getUTCMinutes(), s: d.getUTCSeconds() }
    : { y: d.getFullYear(), mo: d.getMonth(), da: d.getDate(), h: d.getHours(), mi: d.getMinutes(), s: d.getSeconds() };
  const yday = Math.floor((Date.UTC(g.y, g.mo, g.da) - Date.UTC(g.y, 0, 1)) / 86_400_000) + 1;
  return fmt.replace(/%([HMSdmYyjb%])/g, (_, k: string) =>
    ({ H: pad(g.h), M: pad(g.mi), S: pad(g.s), d: pad(g.da), m: pad(g.mo + 1), Y: String(g.y), y: pad(g.y % 100), j: pad(yday, 3), b: MONTHS[g.mo] ?? '', '%': '%' })[k] ?? '',
  );
}

function family4(tag: string, c: MacroContext): string | null {
  switch (tag) {
    case 'TXTIME': return formatTime(c.txStart ?? c.now, '%H:%M', true);
    case 'LOCALDATE': return formatTime(c.now, '%Y-%m-%d', false);
    default: return null;
  }
}

function special5(tag: string, r: MacroRun): boolean {
  const c = r.ctx;
  switch (tag) {
    case 'CLEARNRR': setField(r, 'nrr', ''); return true;
    case 'INCNRR': {
      // il numero ricevuto piu' uno, con gli stessi zeri davanti
      const m = /^(\D*)(\d+)(.*)$/.exec(fromCutNumbers(c.nrr.trim()));
      const next = m ? `${m[1]}${String(Number(m[2]) + 1).padStart(m[2]?.length ?? 1, '0')}${m[3]}` : '1';
      setField(r, 'nrr', next);
      return true;
    }
    // il nominativo corretto si ritrasmette (dopo averlo sistemato nel campo)
    case 'CORRECTCALLTX': r.text(c.call.toUpperCase()); return true;
    case 'SAVESHAUTO': r.act({ type: 'saveQso' }); r.act({ type: 'clearQso' }); return true;
    default: return false;
  }
}

function action5(tag: string, arg: string | undefined): MacroAction | null {
  switch (tag) {
    case 'SAVE':
    case 'SAVESH': return { type: 'saveQso' };
    case 'ERASE': return { type: 'ignoreBefore', at: 'erase' };
    case 'READLOG':
    case 'REFRESH':
    case 'WKD':
    case 'PRINTQSL':
    case 'QSLPRINT':
    case 'SENDEQSL':
    case 'SENDEQSLNP':
    case 'SENDSPOT':
    case 'QSOCMDDDE': return cmd(tag, arg);
    default: return null;
  }
}

function family6(tag: string, arg: string | undefined, c: MacroContext): string | null {
  switch (tag) {
    // la parola TEST nella chiamata di un contest: solo con un contest attivo
    case 'TEST': return c.inContest ? 'TEST' : '';
    // l'identificazione in CW alla fine: Aurora non mescola la CW negli altri modi, il nominativo va in coda al testo
    case 'CWID': {
      const id = arg === undefined || arg.trim().toUpperCase() === 'DEFAULT' ? `DE ${c.mycall.toUpperCase()}` : arg;
      return c.modeName === 'CW' ? '' : ` ${id}`;
    }
    // RTTY: il motore manda da solo LTRS e FIGS dove servono (anche dopo ogni spazio)
    case 'LETTERS':
    case 'NUMBERS': return '';
    default: return null;
  }
}

/** I valori di `<GET x>`. */
function getValue(what: string, c: MacroContext): string {
  switch (what) {
    case 'SN':
    case 'SNR': return c.snr != null ? String(Math.round(c.snr)) : '?';
    case 'OFFSET':
    case 'AUDIOFQ': return c.rxHz != null ? String(Math.round(c.rxHz)) : '?';
    case 'RATE':
    case 'BAUDRATE':
    case 'WPM': return c.rate ?? '?';
    case 'MODE': return c.modeName;
    case 'IMD': return c.imd != null ? String(Math.round(c.imd)) : '?';
    default: return '?';
  }
}

function action6(tag: string, arg: string | undefined): MacroAction | null {
  switch (tag) {
    case 'MODEADD':
    case 'MODETNC': return arg === undefined ? null : { type: 'mode', mode: arg.trim().toUpperCase() };
    case 'LOCKMODE':
    case 'UNLOCKMODE':
    case 'LOCKRX':
    case 'UNLOCKRX':
    case 'LOCKRXTOGGLE':
    case 'MODEMACROS':
    case 'WORDMODE':
    case 'CAPITALSONLY':
    case 'REVERSEPADDLE':
    case 'SNAPNOW':
    case 'INV':
    case 'INVON':
    case 'INVOFF':
    case 'FILTER':
    case 'BROADCASTMODE':
    case 'TOGGLECHANNEL':
    case 'ESCAPE':
    case 'PTT':
    case 'SPZOOM':
    case 'SPSHIFT':
    case 'AMPLITUDE':
    case 'ZFQ':
    case 'FQB': return cmd(tag, arg);
    default: return null;
  }
}

/** I byte esadecimali scritti nella macro (`FE FE 94 E0`) nella forma che Hamlib manda cosi' com'e' (`\0xFE\0xFE...`). */
export function hexToCat(text: string): string | null {
  const clean = text.replace(/0x/gi, '').replace(/[\s,;$-]/g, '');
  if (clean === '' || clean.length % 2 !== 0 || !/^[0-9a-f]+$/i.test(clean)) return null;
  return (clean.match(/../g) ?? []).map((b) => `\\0x${b.toUpperCase()}`).join('');
}

function action7(tag: string, arg: string | undefined): MacroAction | null {
  switch (tag) {
    case 'CATCMDHEX': {
      const raw = hexToCat(arg ?? '');
      return raw === null ? cmd('CATBAD', arg) : { type: 'cat', cmd: raw };
    }
    // CATCMDX accetta gia' le sequenze \0xNN di Hamlib; senza DDE CATCMDDDE manda il comando alla radio di Aurora
    case 'CATCMDX':
    case 'CATCMDDDE': return arg === undefined || arg.trim() === '' ? null : { type: 'cat', cmd: arg.trim() };
    case 'CATWAIT':
    case 'CATTRANSVERTER':
    case 'CATENABLEVFOSYNC':
    case 'CATDISABLEVFOSYNC':
    case 'CATVFOSYNCSHIFT':
    case 'CATVFOSYNCONCE':
    case 'TOGGLECATPORT':
    case 'TOGGLEPTTPORT':
    case 'TOGGLESOUNDDEV':
    case 'INPUTVOLUME':
    case 'OUTPUTVOLUME': return cmd(tag, arg);
    default: return null;
  }
}

function action8(tag: string, arg: string | undefined): MacroAction | null {
  switch (tag) {
    case 'WKTEXT':
    case 'WK': return arg === undefined || arg.trim() === '' ? null : cmd('WKTEXT', arg);
    case 'WKHEX':
    case 'WKPOT':
    case 'WKNOPOT': return cmd(tag, arg);
    default: return null;
  }
}







/** I comandi che cambiano i dati della macro o espandono altro testo: `true` se il comando e' stato gestito. */
export function special(tag: string, arg: string | undefined, r: MacroRun): boolean {
  return (
    special1(tag, arg, r) ||
    special2(tag, r) ||
    special3(tag, arg, r) ||
    special5(tag, r) ||
    false
  );
}

/** Il valore di un comando di testo in piu', o `null`. */
export function extraText(tag: string, arg: string | undefined, c: MacroContext): string | null {
  if (tag.startsWith('GET ')) return getValue(tag.slice(4).trim(), c);
  return (
    family1(tag, arg, c) ??
    family3(tag, c) ??
    family4(tag, c) ??
    family6(tag, arg, c) ??
    null
  );
}

/** L'azione di un comando in piu', o `null`. */
export function extraAction(tag: string, arg: string | undefined): MacroAction | null {
  if (tag === 'SAVESELTEXT') return cmd('SAVESELTEXT');
  if (tag === 'TIMER') return cmd('TIMER', arg);
  if (tag === 'STARTTIME' && arg !== undefined) return cmd('STARTTIME', arg);
  // <SET BW:250>, <SET TONES:16>, <SET SUBMODE:MFSK32>: il parametro del modo
  if (tag.startsWith('SET ')) return cmd('SET', `${tag.slice(4).trim()}:${arg ?? ''}`);
  if (tag === 'WKDIGNOREBEFORE') {
    const at = arg === undefined ? null : parseIgnoreBefore(arg);
    return at === null ? null : { type: 'ignoreBefore', at };
  }
  return (
    action5(tag, arg) ??
    action6(tag, arg) ??
    action7(tag, arg) ??
    action8(tag, arg) ??
    null
  );
}
