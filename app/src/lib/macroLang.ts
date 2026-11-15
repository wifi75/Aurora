/**
 * Il linguaggio delle macro: uno script è testo con comandi fra `<` e `>` in maiuscolo. I comandi di testo (`<CALL>`,
 * `<MYCALL>`, `<RSTS>`...) diventano le parole dei dati; i comandi di programma (`<TX>`, `<RX>`, `<MODE:RTTY>`, `<SAVEQSO>`...) si
 * raccolgono in un elenco di azioni che la pagina esegue.
 */
import { formatSerial } from './contestSerial';
import { parseIgnoreBefore } from './logWindow';
import { FILE_TAGS, beforeMs, extraAction, extraText, fileOf, formatTime, mainCall, special, type MacroRun } from './macroExtra';

export { mainCall };

/** Il QSO precedente (dal log del contest attivo o dal log): per `<PREVNAME>`, `<PREVQTH>`, `<LASTCALL>`... */
export interface PrevQso {
  call: string;
  name?: string;
  qth?: string;
  rsts?: string;
  rstr?: string;
  nrs?: string;
  nrr?: string;
  /** Istante del QSO (ms UTC). */
  ms?: number | null;
}

/** I dati con cui si riempiono i comandi di testo. */
export interface MacroContext {
  mycall: string;
  myname: string;
  myqth: string;
  mywwloc: string;
  /** Il corrispondente. */
  call: string;
  name: string;
  qth: string;
  wwloc: string;
  rstr: string;
  rsts: string;
  notes: string;
  /** Scambio ricevuto e da mandare (contest). */
  nrr: string;
  nrs: string;
  /** Inizio del QSO, se gia' fissato. */
  startTime: Date | null;
  now: Date;
  /** Frequenza della radio in Hz (0 se non nota). */
  dialHz: number;
  band: string;
  /** Il modo in uso, come si legge a schermo (PSK31, RTTY...). */
  modeName: string;
  /** Il numero progressivo del contest (`<QSONR>`, almeno tre cifre: 001). */
  qsoNumber: number;
  /** L'ultimo QSO con lo stesso nominativo nel log (per `<ANYQSOBEFOREDATE>`, `<ANYQSOBEFOREBAND>`, `<ANYQSOBEFOREMODE>`): senza, i tre comandi restano vuoti. */
  lastQso?: { date: string; band: string; mode: string } | null;
  /** Dal log del contest attivo: lo scambio mandato nell'ultimo QSO (`<PREVNRS>`) e il QSO precedente con questo nominativo
   *  (`<QSOBEFORETIME>`, `<QSOBEFOREDATE>`, ora e data UTC). Senza contest attivo restano vuoti. */
  contestPrev?: { lastSent: string; beforeMs: number | null; lastCall?: string; lastRst?: string } | null;
  /** Il contenuto dei file chiesti con `<TEXTFILE:nome>` (e `<FILE:>`, `<RANDOM:>`, `<INIFILE:>`, `<SCRIPT:>`), gia' letti. */
  files?: Record<string, string>;
  /** Le macro dell'utente per nome in maiuscolo: `<F1>`, `<gm>`... dentro un'altra macro. */
  macros?: ReadonlyMap<string, string>;
  /** Il testo selezionato nella finestra di ricezione (`<SELCALL>`, `<SELRXTEXT>`...). */
  selection?: string;
  /** L'ultimo QSO del log (`<PREVNAME>`, `<PREVQTH>`, `<LASTCALL>`...). */
  prevQso?: PrevQso | null;
  /** Il QSO gia' fatto con questo nominativo sulla stessa banda e nello stesso modo (`<QSOBEFORE>`...). */
  before?: { ms: number; band: string; mode: string } | null;
  /** La lingua delle frasi automatiche (`<FORCELANGUAGE:xx>`, `<LANG_xx>`); vuota = dal nominativo. */
  lang?: string;
  /** Quando e' partita l'ultima trasmissione (`<TXTIME>`). */
  txStart?: Date | null;
  /** Frequenza audio di ricezione, S/N, IMD e velocita' del segnale (`<GET OFFSET>`, `<GET SN>`, `<GET RATE>`). */
  rxHz?: number;
  snr?: number | null;
  imd?: number | null;
  rate?: string;
  /** Un contest e' attivo (`<TEST>`). */
  inContest?: boolean;
  /** Il nominativo preso dalla ricezione per `<GRABCALL>` (anche dentro una macro annidata). */
  grabbed?: string;
  /** Numeri a caso (`<RANDOM:file>`); di base `Math.random`. */
  rand?: () => number;
}

export type MacroAction =
  | { type: 'tx' }
  | { type: 'txe' }
  | { type: 'txToggle' }
  | { type: 'rx'; clear: boolean }
  | { type: 'rxe' }
  | { type: 'clearTx' }
  | { type: 'clearRx' }
  | { type: 'clearQso' }
  | { type: 'clearWindow' }
  | { type: 'fixStartTime' }
  | { type: 'saveQso' }
  /** La finestra dei QTC del WAE: mandare o ricevere una serie. */
  | { type: 'qtc'; dir: 'send' | 'receive' }
  | { type: 'mode'; mode: string }
  | { type: 'baud'; baud: number }
  | { type: 'shift'; hz: number }
  /** Velocita' della CW: assoluta o relativa (`+n`, `-n`). */
  | { type: 'wpm'; wpm: number; relative: boolean }
  /** Porta la radio in modo che il segnale cada su questa frequenza (kHz). */
  | { type: 'fq'; khz: number }
  /** Gira il rotore: gradi, o `SP` / `LP` verso la stazione del QSO per la via corta o lunga. */
  | { type: 'rotor'; az: number | 'SP' | 'LP' }
  | { type: 'editMacro'; name: string }
  | { type: 'alarm'; text: string }
  | { type: 'audioFq'; hz: number; relative: boolean }
  /** Auto CQ: `toggle` accende o spegne (o riprende se e' in pausa), `start` lo accende, `resume` riprende quello fermato. */
  | { type: 'autoCq'; how: 'toggle' | 'start' | 'resume' }
  /** Pausa fra due chiamate dell'Auto CQ, in secondi. */
  | { type: 'acqDelay'; seconds: number }
  /** L'ascolto sulla frequenza di trasmissione. */
  | { type: 'jumpToTx' }
  /** La frequenza di ricezione al multiplo piu' vicino di questi Hz. */
  | { type: 'align'; hz: number }
  | { type: 'tune'; seconds: number | null }
  | { type: 'beep' }
  | { type: 'seek'; direction: 'left' | 'right' }
  | { type: 'bookmark' }
  /** AFC, Snap, Squelch e Lock: acceso, spento o commutato. */
  | { type: 'toggle'; what: 'afc' | 'snap' | 'squelch' | 'lock'; value: boolean | 'toggle' }
  | { type: 'scan'; run: boolean }
  /** Salva (`write`) o rilegge le impostazioni nella memoria n (1-20). */
  | { type: 'memory'; write: boolean; slot: number }
  | { type: 'modeSettings' }
  /** I segni del log contano solo i QSO da questo istante (ms UTC); `erase` toglie il limite. */
  | { type: 'ignoreBefore'; at: number | 'erase' }
  /** I segni del log contano solo gli ultimi n minuti (0 = spento). */
  | { type: 'period'; minutes: number }
  | { type: 'threshold' }
  | { type: 'cursor'; target: string }
  | { type: 'macros'; file: string }
  /** `<CATCMD:...>`: una stringa CAT mandata alla radio cosi' com'e'. */
  | { type: 'cat'; cmd: string }
  /** `<EXEC:...>`: avvierebbe un programma esterno; Aurora non lo fa e lo dice (non supportato per sicurezza). */
  | { type: 'exec'; file: string }
  /** `<GRABCALL>`: il nominativo preso dal testo ricevuto va nel QSO (gia' cercato prima dell'esecuzione, vuoto se non c'era). */
  | { type: 'grabCall'; call: string }
  /** Un comando eseguito dalla pagina (vedi `macroRuntime.ts`): nome e argomento. */
  | { type: 'cmd'; tag: string; arg: string }
  /** Un campo del QSO cambiato dalla macro (`<SELCALL>`, `<LASTCALL>`, `<INCNRR>`, `<CALL?>`...). */
  | { type: 'setField'; field: 'call' | 'name' | 'qth' | 'rsts' | 'rstr' | 'nrs' | 'nrr' | 'notes'; value: string }
  /** Un comando che Aurora non conosce ancora: resta nell'elenco per dirlo all'utente. */
  | { type: 'unknown'; tag: string };

/** Un pezzo dello script nell'ordine in cui compare: testo gia' riempito o un comando di programma. */
export type MacroStep = { text: string } | { action: MacroAction };

export interface MacroResult {
  /** Il testo da trasmettere (o da mettere nella casella di trasmissione). */
  text: string;
  actions: MacroAction[];
  /** Testo e comandi nell'ordine dello script: conta dove sta un `<RX>` o un `<CLEARTXWINDOW>` (vedi `macroPlan.ts`). */
  steps: MacroStep[];
}

const pad = (n: number, w = 2) => String(n).padStart(w, '0');
const utcDate = (d: Date) => `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())}`;
const utcTime = (d: Date) => `${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}`;

/** Saluto secondo l'ora: GM, GA, GE (breve) o la frase lunga. */
function greeting(now: Date, long: boolean): string {
  const h = now.getUTCHours();
  const slot = h < 12 ? 0 : h < 18 ? 1 : 2;
  return long ? ['Good Morning', 'Good Afternoon', 'Good Evening'][slot]! : ['GM', 'GA', 'GE'][slot]!;
}

function fixed(value: number, digits: string | undefined): string {
  const n = digits === undefined ? 0 : Math.max(0, Math.min(6, Number.parseInt(digits, 10) || 0));
  return value.toFixed(n);
}

/** Il valore di un comando di testo, o `null` se non e' un comando di testo. */
function textTag(tag: string, arg: string | undefined, c: MacroContext): string | null {
  switch (tag) {
    // segni di procedura: in CW il motore li manda come un segno solo; negli altri modi restano lettere
    case 'AR':
    case 'BT':
    case 'KN':
    case 'SK':
      return c.modeName === 'CW' ? `<${tag}>` : tag;
    case 'CALL': return c.call.toUpperCase();
    case 'NAME': return c.name;
    case 'QTH': return c.qth;
    case 'WWLOC': return c.wwloc.toUpperCase();
    case 'RSTR': return c.rstr;
    case 'RSTS': return c.rsts;
    case 'NOTES': return c.notes;
    case 'NRR': return c.nrr;
    case 'NRS': return c.nrs;
    // con un formato (`<TIME:%H%M>`, `<DATE:%d/%m/%Y>`) data e ora si scrivono cosi'
    case 'DATE': return arg ? formatTime(c.now, arg, true) : utcDate(c.now);
    case 'TIME': return arg ? formatTime(c.now, arg, true) : utcTime(c.now);
    case 'LOCALTIME': return arg ? formatTime(c.now, arg, false) : `${pad(c.now.getHours())}:${pad(c.now.getMinutes())}`;
    // <STARTTIME:hhmm> senza `%` fissa l'inizio del QSO (azione)
    case 'STARTTIME': return arg === undefined ? utcTime(c.startTime ?? c.now) : arg.includes('%') ? formatTime(c.startTime ?? c.now, arg, true) : null;
    case 'CTIME': return arg ? formatTime(c.now, arg, true) : `${pad(c.now.getUTCHours())}${pad(c.now.getUTCMinutes())}`;
    case 'MYCALL': return c.mycall.toUpperCase();
    case 'MYMAINCALL': return mainCall(c.mycall);
    case 'MYNAME': return c.myname;
    case 'MYQTH': return c.myqth;
    case 'MYWWLOC': return c.mywwloc.toUpperCase();
    case 'CR': return '\r';
    case 'CRLF': return '\r\n';
    case 'LF': return '\n';
    case 'TAB': return '\t';
    case '[': return '<';
    case ']': return '>';
    case 'GA': return greeting(c.now, false);
    case 'GAL': return greeting(c.now, true);
    case 'KHZ': return c.dialHz > 0 ? fixed(c.dialHz / 1000, arg) : '';
    case 'MHZ': return c.dialHz > 0 ? fixed(c.dialHz / 1_000_000, arg) : '';
    case 'BAND': return c.band;
    case 'MODE': return arg === undefined ? c.modeName : null;
    case 'QSONR': return formatSerial(c.qsoNumber);
    case 'VER': return 'Aurora';
    // l'ultimo QSO con questo nominativo, dal log
    case 'ANYQSOBEFOREDATE': return c.lastQso?.date ?? '';
    case 'ANYQSOBEFOREBAND': return c.lastQso?.band ?? '';
    case 'ANYQSOBEFOREMODE': return c.lastQso?.mode ?? '';
    // dal log del contest attivo
    case 'PREVNRS': return c.contestPrev?.lastSent ?? '';
    // fuori dal contest: dal log
    case 'PREVCALL': return c.contestPrev?.lastCall || (c.prevQso?.call ?? '').toUpperCase();
    case 'PREVRSTS': return c.contestPrev?.lastRst || (c.prevQso?.rsts ?? '');
    case 'QSOBEFORETIME': {
      const ms = beforeMs(c);
      return ms != null ? utcTime(new Date(ms)) : '';
    }
    case 'QSOBEFOREDATE': {
      const ms = beforeMs(c);
      return ms != null ? utcDate(new Date(ms)) : '';
    }
    // il testo di un file della cartella dei testi delle macro
    case 'TEXTFILE': return arg === undefined ? null : (c.files?.[arg.trim()] ?? '');
    // l'IMD del segnale PSK ricevuto: Aurora non la misura, il comando non scrive niente. `<SP>` segue i comandi per la radio e non scrive niente
    case 'IMD':
    case 'SP': return '';
    default: return null;
  }
}

const number = (v: string | undefined): number | null => {
  if (v === undefined) return null;
  const n = Number(v.replace(',', '.'));
  return Number.isFinite(n) ? n : null;
};

/** Il comando di programma come azione, o `null` se non lo e'. */
function actionOf(tag: string, arg: string | undefined): MacroAction | null {
  switch (tag) {
    case 'TX': return { type: 'tx' };
    case 'TXE': return { type: 'txe' };
    case 'TXTOGGLE': return { type: 'txToggle' };
    case 'RX': return { type: 'rx', clear: false };
    case 'RXANDCLEAR':
    case 'RXC': return { type: 'rx', clear: true };
    case 'RXE': return { type: 'rxe' };
    case 'CLEARTXWINDOW': return { type: 'clearTx' };
    case 'CLEARRXWINDOW': return { type: 'clearRx' };
    case 'CLEARQSO': return { type: 'clearQso' };
    case 'CLEARWINDOW': return { type: 'clearWindow' };
    case 'FIXSTARTTIME': return { type: 'fixStartTime' };
    case 'SAVEQSO': return { type: 'saveQso' };
    case 'QTCSEND': return { type: 'qtc', dir: 'send' };
    case 'QTCREAD': return { type: 'qtc', dir: 'receive' };
    // i nomi dei file di macro importati: <STATS:SendQTC>, <STATS:ReadQTC>
    case 'STATS': {
      const a = (arg ?? '').trim().toUpperCase();
      return a === 'SENDQTC' ? { type: 'qtc', dir: 'send' } : a === 'READQTC' ? { type: 'qtc', dir: 'receive' } : null;
    }
    case 'AUTOCQ':
    case 'ACQ': return { type: 'autoCq', how: 'toggle' };
    case 'ASAUTOCQ': return { type: 'autoCq', how: 'start' };
    case 'AUTOCQRESUME': return { type: 'autoCq', how: 'resume' };
    case 'ACQDELAY': {
      const n = number(arg);
      return n === null || n < 0 || n > 600 ? null : { type: 'acqDelay', seconds: n };
    }
    case 'JUMPTOTXFQ': return { type: 'jumpToTx' };
    case 'ALIGN': {
      const hz = number(arg);
      return hz === null || hz <= 0 ? null : { type: 'align', hz };
    }
    case 'BEEP': return arg === 'ON' ? { type: 'beep' } : null;
    case 'SEEKLEFT': return { type: 'seek', direction: 'left' };
    case 'SEEKRIGHT': return { type: 'seek', direction: 'right' };
    case 'BOOKMARK': return { type: 'bookmark' };
    case 'EXEC': return { type: 'exec', file: arg ?? '' };
    case 'GRABCALL': return { type: 'grabCall', call: (arg ?? '').trim().toUpperCase() };
    case 'AFCON': return { type: 'toggle', what: 'afc', value: true };
    case 'AFCOFF': return { type: 'toggle', what: 'afc', value: false };
    case 'AFC': return { type: 'toggle', what: 'afc', value: 'toggle' };
    case 'SNAPON': return { type: 'toggle', what: 'snap', value: true };
    case 'SNAPOFF': return { type: 'toggle', what: 'snap', value: false };
    case 'SNAP': return { type: 'toggle', what: 'snap', value: 'toggle' };
    case 'SQUELCHON': return { type: 'toggle', what: 'squelch', value: true };
    case 'SQUELCHOFF': return { type: 'toggle', what: 'squelch', value: false };
    case 'SQUELCH': return { type: 'toggle', what: 'squelch', value: 'toggle' };
    // «Lock» e' la frequenza di trasmissione fissa: LOCKTX
    case 'LOCKTX': return { type: 'toggle', what: 'lock', value: true };
    case 'UNLOCKTX': return { type: 'toggle', what: 'lock', value: false };
    case 'LOCKTXTOGGLE': return { type: 'toggle', what: 'lock', value: 'toggle' };
    case 'STARTSCAN': return { type: 'scan', run: true };
    case 'STOPSCAN': return { type: 'scan', run: false };
    case 'MODESETTINGS': return { type: 'modeSettings' };
    case 'IGNOREBEFORE':
    case 'WKDIGNOREBEFORE': {
      const at = arg === undefined ? null : parseIgnoreBefore(arg);
      return at === null ? null : { type: 'ignoreBefore', at };
    }
    case 'PERIOD': {
      const n = number(arg);
      return n === null || n < 0 ? null : { type: 'period', minutes: Math.trunc(n) };
    }
    case 'THRESHOLD': return { type: 'threshold' };
    case 'MEMW':
    case 'MEMR': {
      const slot = number(arg);
      return slot === null || !Number.isInteger(slot) || slot < 1 || slot > 20 ? null : { type: 'memory', write: tag === 'MEMW', slot };
    }
    case 'TUNE': return { type: 'tune', seconds: number(arg) };
    case 'MODE': return arg === undefined ? null : { type: 'mode', mode: arg.trim().toUpperCase() };
    case 'CURSOR': return arg === undefined ? null : { type: 'cursor', target: arg.trim().toUpperCase() };
    case 'MACROS': return arg === undefined ? null : { type: 'macros', file: arg.trim() };
    case 'CATCMD': return arg === undefined || arg.trim() === '' ? null : { type: 'cat', cmd: arg.trim() };
    case 'BAUDRATE': {
      const baud = number(arg);
      return baud === null || baud <= 0 ? null : { type: 'baud', baud };
    }
    case 'SHIFT': {
      const hz = number(arg);
      return hz === null || hz <= 0 ? null : { type: 'shift', hz };
    }
    case 'FQ': {
      const khz = number(arg);
      return khz === null || khz <= 0 ? null : { type: 'fq', khz };
    }
    case 'SPLIT': return arg === '1' ? { type: 'toggle', what: 'lock', value: true } : arg === '0' ? { type: 'toggle', what: 'lock', value: false } : null;
    case 'ROTOR': {
      const a = (arg ?? '').trim().toUpperCase();
      if (a === 'SP' || a === 'LP') return { type: 'rotor', az: a };
      const az = number(arg);
      return az === null ? null : { type: 'rotor', az: ((Math.round(az) % 360) + 360) % 360 };
    }
    case 'EDITMACRO': return arg === undefined || arg.trim() === '' ? null : { type: 'editMacro', name: arg.trim() };
    case 'ALARM': return arg === undefined ? null : { type: 'alarm', text: arg };
    case 'WPM': {
      const wpm = number(arg);
      return wpm === null || wpm === 0 ? null : { type: 'wpm', wpm, relative: /^[+-]/.test((arg ?? '').trim()) };
    }
    case 'AUDIOFQ': {
      const hz = number(arg);
      return hz === null ? null : { type: 'audioFq', hz, relative: /^[+-]/.test((arg ?? '').trim()) };
    }
    default: return null;
  }
}

/**
 * Un comando: lettere, cifre e qualche segno, con un secondo nome dopo uno spazio (`<SET BW:250>`, `<GET SN>`, `<RXLOG START>`), un `?` in
 * fondo (`<CALL?>`, `<F1?>`) e un argomento dopo i due punti. Le parentesi `<[>` e `<]>` sono comandi di testo.
 */
const TAG = /<(\[|\]|[A-Za-z][A-Za-z0-9_/+-]*(?: [A-Za-z0-9_]+)?)(\?)?(?::([^<>]*))?>/g;

/** Al massimo 10 macro una dentro l'altra (come nell'originale): oltre ci si ferma con un avviso. */
export const MAX_NESTING = 10;
/** I campi che con `?` in fondo si chiedono se sono vuoti. */
export const ASKABLE = ['CALL', 'RSTS', 'RSTR', 'NRS', 'NRR'] as const;

/** Esegue uno script: restituisce il testo e le azioni, nell'ordine in cui compaiono. Le macro dell'utente nominate dentro (`<F1>`) si espandono. */
export function runMacro(script: string, ctx: MacroContext): MacroResult {
  const actions: MacroAction[] = [];
  const steps: MacroStep[] = [];
  let hidden = false;
  let stopped = false;
  const c: MacroContext = { ...ctx };
  const act = (a: MacroAction) => {
    actions.push(a);
    steps.push({ action: a });
  };
  const text = (t: string) => {
    if (!hidden && t !== '') steps.push({ text: t });
  };
  const expand = (part: string, depth: number) => {
    if (depth > MAX_NESTING) {
      if (!stopped) act({ type: 'alarm', text: 'MACRO: max 10' });
      stopped = true;
      return;
    }
    const run: MacroRun = { ctx: c, text, act, expand: (s) => expand(s, depth + 1), setHidden: (h) => { hidden = h; } };
    let last = 0;
    for (const m of part.matchAll(TAG)) {
      if (stopped) return;
      const at = m.index ?? 0;
      text(part.slice(last, at));
      last = at + m[0].length;
      const raw = m[1] ?? '';
      const ask = m[2] === '?';
      const arg = m[3];
      const tag = raw.toUpperCase();
      // una macro dell'utente: il suo testo si espande al suo posto; con `?` si apre per modificarla
      const user = arg === undefined ? c.macros?.get(tag) : undefined;
      if (user !== undefined) {
        if (ask) act({ type: 'editMacro', name: raw });
        else expand(user, depth + 1);
        continue;
      }
      // il nome di un tasto senza macro: niente da eseguire (con `?` si apre per scriverla)
      if (arg === undefined && /^(?:CTRL-|SHIFT-|CTRL-SHIFT-)?F(?:[1-9]|1[0-2])$/.test(tag)) {
        if (ask) act({ type: 'editMacro', name: raw });
        continue;
      }
      if (special(tag, arg, run)) continue;
      // <GRABCALL> dentro una macro annidata: il nominativo gia' preso dalla pagina
      if (tag === 'GRABCALL' && arg === undefined && c.grabbed) {
        c.call = c.grabbed;
        act({ type: 'grabCall', call: c.grabbed });
        continue;
      }
      const value = textTag(tag, arg, c) ?? extraText(tag, arg, c);
      if (value !== null) {
        text(value);
        continue;
      }
      const action = actionOf(tag, arg) ?? extraAction(tag, arg);
      // un nome in minuscolo che non e' una macro ne' un comando resta testo (per esempio <br>)
      if (action === null && raw !== tag) {
        text(m[0]);
        continue;
      }
      act(action ?? { type: 'unknown', tag: arg === undefined ? tag : `${tag}:${arg}` });
    }
    text(part.slice(last));
  };
  expand(script, 0);
  const all = steps.map((st) => ('text' in st ? st.text : '')).join('');
  return { text: tidyText(all), actions, steps };
}

/** Che cosa serve leggere prima di eseguire uno script (anche dentro le macro che nomina): file, log, contest, selezione, valori da chiedere. */
export interface MacroNeeds {
  files: string[];
  log: boolean;
  contest: boolean;
  selection: boolean;
  grab: boolean;
  ask: (typeof ASKABLE)[number][];
}

export function macroNeeds(script: string, macros?: ReadonlyMap<string, string>): MacroNeeds {
  const files = new Set<string>();
  const ask = new Set<(typeof ASKABLE)[number]>();
  const seen = new Set<string>();
  let all = '';
  const walk = (part: string, depth: number) => {
    if (depth > MAX_NESTING) return;
    all += part;
    for (const m of part.matchAll(TAG)) {
      const tag = (m[1] ?? '').toUpperCase();
      const arg = m[3];
      const user = arg === undefined ? macros?.get(tag) : undefined;
      if (user !== undefined) {
        if (!seen.has(tag)) {
          seen.add(tag);
          walk(user, depth + 1);
        }
        continue;
      }
      if (arg !== undefined && (FILE_TAGS as readonly string[]).includes(tag)) {
        const f = fileOf(tag, arg);
        if (f !== '') files.add(f);
      }
      if (m[2] === '?' && (ASKABLE as readonly string[]).includes(tag)) ask.add(tag as (typeof ASKABLE)[number]);
      // ONQSOBEFORE:macro e i saluti con le macro gm/ga/ge/gn
      const named = tag === 'ONQSOBEFORE' ? (arg ?? '').trim().toUpperCase() : null;
      for (const n of named ? [named] : /^G[MAEN]$/.test(tag) ? ['GM', 'GA', 'GE', 'GN'] : []) {
        const t = macros?.get(n);
        if (t !== undefined && !seen.has(n)) {
          seen.add(n);
          walk(t, depth + 1);
        }
      }
    }
  };
  walk(script, 0);
  return {
    files: [...files],
    log: /<(?:ANYQSOBEFORE(?:DATE|BAND|MODE)|PREV[A-Z0-9]+|LASTCALL|QSOBEFORE[A-Z0-9]*|TIME1|ONQSOBEFORE:[^<>]*)>/i.test(all),
    contest: /<(?:PREVNRS|PREVCALL|PREVRSTS|QSOBEFORE[A-Z0-9]*|TIME1|ONQSOBEFORE:[^<>]*|LASTCALL)>/i.test(all),
    selection: /<(?:SEL[A-Z]+|SAVESELTEXT)>/i.test(all),
    grab: /<GRABCALL>/i.test(all),
    ask: [...ask],
  };
}

/** Il testo senza spazi doppi, senza spazi in testa e in coda e senza spazi prima di un a capo. */
export function tidyText(text: string): string {
  return text.replace(/ {2,}/g, ' ').replace(/^ /, '').replace(/ $/, '').replace(/ ([\r\n])/g, '$1');
}

/** I nomi dei 48 tasti, nell'ordine: F1-F12, poi Ctrl, Shift e Ctrl-Shift. */
export const MACRO_ROWS = ['', 'Ctrl-', 'Shift-', 'Ctrl-Shift-'] as const;

export function keyNames(): string[] {
  return MACRO_ROWS.flatMap((p) => Array.from({ length: 12 }, (_, i) => `${p}F${i + 1}`));
}

/** I nomi delle macro evento (non legate a un tasto). */
export const EVENT_MACROS = ['OnStart', 'OnEnd', 'OnTX', 'OnCR', 'OnClickAny', 'OnAlarm', 'OnSP0', 'OnSP1', 'Gray+', 'Hello', 'AutoCqText'] as const;

/** Il nome della macro per un tasto premuto (`F5` con i modificatori), o `null` se non e' un tasto di funzione. */
export function macroNameForKey(e: { key: string; ctrlKey: boolean; shiftKey: boolean }): string | null {
  const m = /^F([1-9]|1[0-2])$/.exec(e.key);
  // solo F1-F12 senza modificatori: Ctrl e Shift non sono assegnati (le macro delle altre righe si usano col clic)
  if (!m || e.ctrlKey || e.shiftKey) return null;
  return `F${m[1]}`;
}

export interface Macro {
  name: string;
  label: string;
  text: string;
  color: number | null;
}

/** L'insieme in uso: le macro del modo sostituiscono quelle di base con lo stesso nome; le altre restano. */
export function effectiveMacros(base: readonly Macro[], forMode: readonly Macro[] | undefined, merge = true): Map<string, Macro> {
  const map = new Map<string, Macro>();
  // senza «Merge» le macro del modo sostituiscono del tutto quelle di base; se il modo non ne ha, restano quelle di base
  if (merge || !forMode || forMode.length === 0) for (const m of base) map.set(m.name, m);
  for (const m of forMode ?? []) map.set(m.name, m);
  return map;
}

/** Il colore per un testo leggibile sullo sfondo `0xRRGGBB`. */
export function textColorFor(color: number | null): string {
  if (color === null) return 'inherit';
  const r = (color >> 16) & 255;
  const g = (color >> 8) & 255;
  const b = color & 255;
  return (r * 299 + g * 587 + b * 114) / 1000 > 140 ? '#101418' : '#ffffff';
}

export function cssColor(color: number | null): string | undefined {
  return color === null ? undefined : `#${(color & 0xffffff).toString(16).padStart(6, '0')}`;
}
