import type { AuroraEvent } from '../api/schemas';

/** Un evento dei modi testuali del flusso (testo ricevuto, stato). */
export type DigiStreamEvent = Extract<AuroraEvent, { type: 'digi' }>;

type Listener = (event: DigiStreamEvent) => void;

const listeners = new Set<Listener>();
const textListeners = new Set<() => void>();

/** Il testo della finestra di ricezione: resta anche lasciando la pagina (ultimi 40000 caratteri). */
const MAX_TEXT = 40000;
let text = '';
/** Accanto al testo, carattere per carattere: '1' = incerto (RTTY, mostrato in chiaro), '0' = certo. */
let weak = '';
/** Quanti caratteri sono usciti dall'inizio del testo da quando la pagina e' aperta: la posizione assoluta di ogni riga, che fa da chiave stabile. */
let dropped = 0;

export function getDigiText(): string {
  return text;
}

export function getDigiWeak(): string {
  return weak;
}

/** La posizione assoluta del primo carattere del testo (cresce quando l'inizio si taglia o si cancella). */
export function getDigiBase(): number {
  return dropped;
}

function notify(): void {
  for (const l of textListeners) l();
}

/** Aggiunge in coda al testo; `mask`: i caratteri incerti ('1'), di base tutti certi. */
function append(add: string, mask = '0'.repeat(add.length)): void {
  text += add;
  weak += mask;
  if (text.length > MAX_TEXT) {
    dropped += text.length - MAX_TEXT;
    text = text.slice(text.length - MAX_TEXT);
    weak = weak.slice(weak.length - MAX_TEXT);
  }
  notify();
}

export function clearDigiText(): void {
  dropped += text.length;
  text = '';
  weak = '';
  provisional = '';
  notify();
}

/** Il testo trasmesso compare nella finestra, in una riga a parte (come in fldigi). */
export function appendDigiTx(sent: string): void {
  const sep = text === '' || text.endsWith('\n') ? '' : '\n';
  append(`${sep}» ${sent.trim()}\n`);
}

/** Una nota nella finestra di ricezione (ora, frequenza della stazione trovata dalla scansione...), su una riga a parte. */
export function appendDigiNote(note: string): void {
  const sep = text === '' || text.endsWith('\n') ? '' : '\n';
  append(`${sep}[${note}]\n`);
}

/** Toglie dalla finestra di ricezione le righe che rispondono a `test` (macro `<BMERASE>`: le righe dei segni). */
export function removeDigiLines(test: (line: string) => boolean): void {
  let outText = '';
  let outWeak = '';
  let at = 0;
  const lines = text.split('\n');
  if (text.endsWith('\n')) lines.pop();
  for (const line of lines) {
    const len = line.length + 1;
    if (!test(line)) {
      outText += `${line}\n`;
      outWeak += weak.slice(at, at + len).padEnd(len, '0');
    }
    at += len;
  }
  // l'ultima riga non aveva l'a capo
  if (!text.endsWith('\n')) {
    outText = outText.slice(0, -1);
    outWeak = outWeak.slice(0, -1);
  }
  dropped += text.length - outText.length;
  text = outText;
  weak = outWeak;
  notify();
}

/** L'eco della trasmissione e' aperta: i caratteri partiti via radio si aggiungono alla riga «» » corrente. */
let echoOpen = false;

/** L'eco dei caratteri appena partiti via radio (evento `tx_sent`), al ritmo della trasmissione. */
function appendTxEcho(raw: string): void {
  let add = '';
  if (!echoOpen) {
    add = `${text === '' || text.endsWith('\n') ? '' : '\n'}» `;
    echoOpen = true;
  }
  for (const c of raw) {
    if (c === '\r') continue;
    add += c === '\n' ? '\n» ' : c;
  }
  append(add);
}

/** Dopo una pausa cosi' (ms) senza caratteri la ricezione va a capo da sola (le stazioni mandano il ritorno a capo all'inizio della
 * trasmissione, dove spesso si perde mentre il ricevitore si aggancia). */
export const RX_GAP_MS = 1500;
/** In CW il testo definitivo esce una parola alla volta, a pause normali di 1-2 secondi anche dentro la stessa chiamata: andare a capo dopo 1,5 s
 * metteva ogni parola su una riga sua (segnalazione #110, registrazione di ON7KEC: pause di 1,0-1,7 s fra le parole, 5 s fra due chiamate). */
export const RX_GAP_CW_MS = 5000;
const rxGap = (mode: string | undefined): number => (mode === 'cw' ? RX_GAP_CW_MS : RX_GAP_MS);
let lastRxAt = 0;
const lastExtraAt = new Map<number, number>();

/** I caratteri ricevuti in coda al testo: i ritorni a capo diventano '\n', le posizioni in `weakAt` (nel testo dell'evento) incerte. */
function appendRx(raw: string, weakAt: readonly number[] | undefined, mode?: string): void {
  const flags = new Set(weakAt ?? []);
  let add = '';
  let mask = '';
  // dopo una pausa si va a capo da soli: ogni chiamata e ogni risposta su una riga sua
  const now = Date.now();
  const before = getDigiText();
  if (lastRxAt > 0 && now - lastRxAt > rxGap(mode) && before !== '' && !before.endsWith('\n') && !/^[\r\n]/.test(raw)) {
    add += '\n';
    mask += '0';
  }
  lastRxAt = now;
  for (let k = 0; k < raw.length; k++) {
    const c = raw[k];
    if (c === '\r') {
      if (raw[k + 1] === '\n') continue;
      add += '\n';
      mask += '0';
    } else {
      add += c;
      mask += c !== '\n' && flags.has(k) ? '1' : '0';
    }
  }
  append(add, mask);
}

/** Chi mostra il testo si iscrive qui: cambia a ogni carattere nuovo. */
export function onDigiText(listener: () => void): () => void {
  textListeners.add(listener);
  return () => {
    textListeners.delete(listener);
  };
}

/** Chi segue lo stato (ascolto, trasmissione) si iscrive qui. */
export function onDigi(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Il testo di ogni finestra RX in piu' (per numero), al massimo `EXTRA_MAX_CHARS` caratteri ciascuna. */
const EXTRA_MAX_CHARS = 6000;
const extraText = new Map<number, string>();
const extraListeners = new Set<() => void>();

export function getExtraText(id: number): string {
  return extraText.get(id) ?? '';
}

/** Svuota il testo di una finestra in piu' (chiusa o spostata su un'altra stazione). */
export function clearExtraText(id: number): void {
  extraText.delete(id);
  for (const l of extraListeners) l();
}

export function onExtraText(listener: () => void): () => void {
  extraListeners.add(listener);
  return () => {
    extraListeners.delete(listener);
  };
}

function appendExtra(id: number, raw: string, mode?: string): void {
  const now = Date.now();
  const old = extraText.get(id) ?? '';
  const last = lastExtraAt.get(id) ?? 0;
  lastExtraAt.set(id, now);
  const gap = last > 0 && now - last > rxGap(mode) && old !== '' && !old.endsWith('\n') && !/^[\r\n]/.test(raw);
  const add = (gap ? '\n' : '') + raw.replace(/\r\n?/g, '\n');
  const next = old + add;
  extraText.set(id, next.length > EXTRA_MAX_CHARS ? next.slice(next.length - EXTRA_MAX_CHARS) : next);
  for (const l of extraListeners) l();
}

/** CW veloce: il testo provvisorio in coda alla finestra principale e a quelle in piu' (sostituito a ogni evento `rx_prov`). */
let provisional = '';
const extraProv = new Map<number, string>();

export function getDigiProvisional(): string {
  return provisional;
}

export function getExtraProvisional(id: number): string {
  return extraProv.get(id) ?? '';
}

export function emitDigi(event: DigiStreamEvent): void {
  if (event.kind === 'rx_prov') {
    const p = (event.text ?? '').replace(/\r\n?/g, '\n');
    if (event.rx_id != null) {
      extraProv.set(event.rx_id, p);
      for (const l of extraListeners) l();
    } else if (p !== provisional) {
      provisional = p;
      notify();
    }
    for (const l of listeners) l(event);
    return;
  }
  // il testo di una finestra RX in piu' va nella sua finestra, non in quella principale
  if (event.kind === 'rx_text' && event.text && event.rx_id != null) {
    appendExtra(event.rx_id, event.text, event.mode);
    for (const l of listeners) l(event);
    return;
  }
  if (event.kind === 'rx_text' && event.text) appendRx(event.text, event.weak, event.mode);
  if (event.kind === 'tx_sent' && event.text) appendTxEcho(event.text);
  if (event.kind === 'tx' && event.state && event.state !== 'sending' && echoOpen) {
    echoOpen = false;
    append('\n');
  }
  for (const l of listeners) l(event);
}
