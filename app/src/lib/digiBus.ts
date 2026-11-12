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

export function getDigiText(): string {
  return text;
}

export function getDigiWeak(): string {
  return weak;
}

function notify(): void {
  for (const l of textListeners) l();
}

/** Aggiunge in coda al testo; `mask`: i caratteri incerti ('1'), di base tutti certi. */
function append(add: string, mask = '0'.repeat(add.length)): void {
  text += add;
  weak += mask;
  if (text.length > MAX_TEXT) {
    text = text.slice(text.length - MAX_TEXT);
    weak = weak.slice(weak.length - MAX_TEXT);
  }
  notify();
}

export function clearDigiText(): void {
  text = '';
  weak = '';
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

/** I caratteri ricevuti in coda al testo: i ritorni a capo diventano '\n', le posizioni in `weakAt` (nel testo dell'evento) incerte. */
function appendRx(raw: string, weakAt: readonly number[] | undefined): void {
  const flags = new Set(weakAt ?? []);
  let add = '';
  let mask = '';
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

export function emitDigi(event: DigiStreamEvent): void {
  if (event.kind === 'rx_text' && event.text) appendRx(event.text, event.weak);
  for (const l of listeners) l(event);
}
