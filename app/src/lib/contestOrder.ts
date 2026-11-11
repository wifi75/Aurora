import type { DecodedMessage } from '../api/schemas';
import type { Valore } from '../api/contest';
import { periodSecs } from './period';

/** Come si ordina l'elenco delle decodifiche del contest: per valore (punti e moltiplicatori) o per ora (le nuove in fondo). */
export type OrdineDecodifiche = 'valore' | 'ora';

/**
 * Quanti periodi (il corrente e i precedenti) restano nell'ordine «Valore»: senza questo limite chi non si sente piu'
 * da minuti restava in cima all'elenco (richiesta di ON7KEC dopo il contest DARC FT4).
 */
export const PERIODI_VALORE = 2;

/** Il valore per l'ordinamento: i doppi in fondo, poi punti e moltiplicatori nuovi. */
export const peso = (v?: Valore) => (!v ? 0 : v.doppio || v.fuori_regola ? -1 : v.punti + 10 * v.molt_nuovi.length);

/**
 * Le decodifiche da mostrare. «Ora»: tutte. «Valore»: solo quelle degli ultimi PERIODI_VALORE periodi,
 * contati dal periodo dell'ultima decodifica (l'ora di una decodifica e' l'inizio del suo periodo).
 */
export function decodificheVisibili(decodes: DecodedMessage[], ordine: OrdineDecodifiche, mode: string): DecodedMessage[] {
  if (ordine === 'ora') return decodes;
  const lenMs = periodSecs(mode) * 1000;
  const top = decodes.reduce((m, d) => Math.max(m, d.time_ms), 0);
  const inizio = Math.floor(top / lenMs) * lenMs - (PERIODI_VALORE - 1) * lenMs;
  return decodes.filter((d) => d.time_ms >= inizio);
}

/**
 * Un messaggio a cui si puo' rispondere: un CQ (o QRZ) oppure uno indirizzato a me. Un QSO fra altre due stazioni no: chi trasmette e'
 * occupato e chiamarlo adesso fa solo perdere un periodo.
 */
export function chiamabile(text: string, mio: string): boolean {
  const w = (text ?? '').trim().toUpperCase().split(/\s+/);
  if (w[0] === 'CQ' || w[0] === 'QRZ') return true;
  const me = mio.trim().toUpperCase();
  return !!me && w[0]?.replace(/[<>]/g, '') === me;
}

/**
 * Ordina le righe: «Valore» prima i messaggi a cui si puo' rispondere (CQ e chiamate a me), poi dal piu' utile (a parita' il segnale piu'
 * forte); «Ora» dalla piu' vecchia alla piu' nuova (la nuova in fondo).
 */
export function ordinaRighe<T extends { d: Pick<DecodedMessage, 'time_ms' | 'snr' | 'text'>; v?: Valore }>(righe: T[], ordine: OrdineDecodifiche, mio = ''): T[] {
  const out = [...righe];
  if (ordine === 'ora') return out.sort((a, b) => a.d.time_ms - b.d.time_ms);
  const c = (x: T) => (chiamabile(x.d.text, mio) ? 1 : 0);
  return out.sort((a, b) => c(b) - c(a) || peso(b.v) - peso(a.v) || b.d.snr - a.d.snr);
}
