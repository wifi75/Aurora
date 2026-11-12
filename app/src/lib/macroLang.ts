/**
 * Il linguaggio delle macro: uno script è testo con comandi fra `<` e `>` in maiuscolo. I comandi di testo (`<CALL>`,
 * `<MYCALL>`, `<RSTS>`...) diventano le parole dei dati; i comandi di programma (`<TX>`, `<RX>`, `<MODE:RTTY>`, `<SAVEQSO>`...) si
 * raccolgono in un elenco di azioni che la pagina esegue. Funzioni e nomi dei comandi ripresi dalla Guida (solo il comportamento).
 */

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
  qsoNumber: number;
  /** L'ultimo QSO con lo stesso nominativo nel log (per `<ANYQSOBEFOREDATE>`, `<ANYQSOBEFOREBAND>`, `<ANYQSOBEFOREMODE>`): senza, i tre comandi restano vuoti. */
  lastQso?: { date: string; band: string; mode: string } | null;
  /** Il contenuto dei file chiesti con `<TEXTFILE:nome>`, gia' letti. */
  files?: Record<string, string>;
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
  | { type: 'autoCq' }
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
  | { type: 'threshold' }
  | { type: 'cursor'; target: string }
  | { type: 'macros'; file: string }
  /** `<CATCMD:...>`: una stringa CAT mandata alla radio cosi' com'e'. */
  | { type: 'cat'; cmd: string }
  /** Un comando che Aurora non conosce ancora: resta nell'elenco per dirlo all'utente. */
  | { type: 'unknown'; tag: string };

export interface MacroResult {
  /** Il testo da trasmettere (o da mettere nella casella di trasmissione). */
  text: string;
  actions: MacroAction[];
}

const pad = (n: number, w = 2) => String(n).padStart(w, '0');
const utcDate = (d: Date) => `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())}`;
const utcTime = (d: Date) => `${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}`;

/** Il nominativo senza i suffissi (/P, /M...) e senza il prefisso di un altro paese. */
export function mainCall(call: string): string {
  const parts = call.toUpperCase().split('/').filter(Boolean);
  if (parts.length <= 1) return parts[0] ?? '';
  // si tiene la parte piu' lunga e con una cifra: quella e' il nominativo vero
  return [...parts].sort((a, b) => Number(/\d/.test(b)) - Number(/\d/.test(a)) || b.length - a.length)[0] ?? '';
}

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
    case 'DATE': return utcDate(c.now);
    case 'TIME': return utcTime(c.now);
    case 'LOCALTIME': return `${pad(c.now.getHours())}:${pad(c.now.getMinutes())}`;
    case 'STARTTIME': return utcTime(c.startTime ?? c.now);
    case 'CTIME': return `${pad(c.now.getUTCHours())}${pad(c.now.getUTCMinutes())}`;
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
    case 'QSONR': return String(c.qsoNumber);
    case 'VER': return 'Aurora';
    // l'ultimo QSO con questo nominativo, dal log
    case 'ANYQSOBEFOREDATE': return c.lastQso?.date ?? '';
    case 'ANYQSOBEFOREBAND': return c.lastQso?.band ?? '';
    case 'ANYQSOBEFOREMODE': return c.lastQso?.mode ?? '';
    // il testo di un file della cartella
    case 'TEXTFILE': return arg === undefined ? null : (c.files?.[arg.trim()] ?? '');
    //  misura l'IMD del segnale PSK ricevuto; Aurora non la misura: il comando non scrive niente. `<SP>` segue i comandi per la radio e non scrive niente
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
    case 'AUTOCQ':
    case 'ACQ': return { type: 'autoCq' };
    case 'BEEP': return arg === 'ON' ? { type: 'beep' } : null;
    case 'SEEKLEFT': return { type: 'seek', direction: 'left' };
    case 'SEEKRIGHT': return { type: 'seek', direction: 'right' };
    case 'BOOKMARK': return { type: 'bookmark' };
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

/** Un comando: lettere maiuscole, cifre e qualche segno, con un argomento dopo i due punti. Le parentesi `<[>` e `<]>` sono comandi di testo. */
const TAG = /<(\[|\]|[A-Z][A-Z0-9_/+-]*)(?::([^<>]*))?>/g;

/** Esegue uno script: restituisce il testo e le azioni, nell'ordine in cui compaiono. */
export function runMacro(script: string, ctx: MacroContext): MacroResult {
  const actions: MacroAction[] = [];
  const text = script.replace(TAG, (_whole, tag: string, arg: string | undefined) => {
    const value = textTag(tag, arg, ctx);
    if (value !== null) return value;
    const action = actionOf(tag, arg);
    if (action) {
      actions.push(action);
      return '';
    }
    actions.push({ type: 'unknown', tag: arg === undefined ? tag : `${tag}:${arg}` });
    return '';
  });
  return { text: text.replace(/ {2,}/g, ' ').replace(/^ /, '').replace(/ $/, '').replace(/ ([\r\n])/g, '$1'), actions };
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
  if (!m) return null;
  return `${e.ctrlKey ? 'Ctrl-' : ''}${e.shiftKey ? 'Shift-' : ''}F${m[1]}`;
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
