import { STRINGS, type Key, type T } from '../i18n';
import { coreText } from './coreText';

/** L'esito dell'ultimo invio di un servizio, come lo manda il core. */
export interface SendStatusLike {
  message: string;
  code?: string;
  params?: Record<string, string>;
}

/**
 * Il testo dell'esito nella lingua dell'interfaccia: se il core manda un codice conosciuto lo si traduce con i suoi valori,
 * altrimenti si ripiega sul testo grezzo (passato dalle traduzioni dei messaggi del core).
 */
export function sendText(st: SendStatusLike, t: T): string {
  const code = st.code ?? '';
  if (code === '') return coreText(st.message, t);
  const p = st.params ?? {};
  // «QSO OK» e simili: tra parentesi solo se c'e'
  const vars: Record<string, string> = { calls: p.calls ?? '', detail: p.detail ?? '', time: p.time ?? '', d: p.detail ? ` (${p.detail})` : '' };
  if (code === 'clublog_retry_network' || code === 'hrdlog_retry_network') vars.detail = coreText(vars.detail, t);
  const key = (code === 'clublog_rejected' && !vars.detail ? 'send.clublog_rejected.plain' : `send.${code}`) as Key;
  if (!(key in STRINGS)) return coreText(st.message, t);
  return t(key, vars);
}
