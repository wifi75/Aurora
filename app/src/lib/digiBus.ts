import type { AuroraEvent } from '../api/schemas';

/** Un evento dei modi testuali del flusso (testo ricevuto, stato). */
export type DigiStreamEvent = Extract<AuroraEvent, { type: 'digi' }>;

type Listener = (event: DigiStreamEvent) => void;

const listeners = new Set<Listener>();
const textListeners = new Set<() => void>();

/** Il testo della finestra di ricezione: resta anche lasciando la pagina (ultimi 40000 caratteri). */
const MAX_TEXT = 40000;
let text = '';

export function getDigiText(): string {
  return text;
}

function setText(next: string): void {
  text = next.length > MAX_TEXT ? next.slice(next.length - MAX_TEXT) : next;
  for (const l of textListeners) l();
}

export function clearDigiText(): void {
  setText('');
}

/** Il testo trasmesso compare nella finestra, in una riga a parte (come in fldigi). */
export function appendDigiTx(sent: string): void {
  const sep = text === '' || text.endsWith('\n') ? '' : '\n';
  setText(`${text}${sep}» ${sent.trim()}\n`);
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
  if (event.kind === 'rx_text' && event.text) setText(text + event.text.replace(/\r\n?/g, '\n'));
  for (const l of listeners) l(event);
}
