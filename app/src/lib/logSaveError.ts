import { ApiError } from '../api/client';
import type { T } from '../i18n';
import { bandOf } from './format';

/** Il motivo, in parole, per cui il log ha rifiutato (o non ha potuto ricevere) un QSO: «QSO non salvato: ...». */
export function saveErrorText(e: unknown, t: T): string {
  if (e instanceof ApiError && e.code === 'bad_field') {
    // il core dice quale campo non va: la banda e' il caso piu' comune (radio senza CAT, frequenza non nota)
    if (e.detail === 'band') return t('digi.log.saveNoBand');
    return t('digi.log.saveBadField', { field: e.detail ?? '' });
  }
  if (e instanceof ApiError && e.code === 'local_only') return t('digi.log.saveLocalOnly');
  // il testo d'errore gia' tradotto che manda il core, se c'e'
  const msg = e instanceof ApiError ? (e.detail ?? e.code) : e instanceof Error ? e.message : '';
  return t('digi.log.saveFail', { msg: msg === '' ? '?' : msg });
}

/**
 * La banda con cui registrare il QSO: quella della frequenza della radio se e' nota, altrimenti quella scelta a mano con i pulsanti
 * delle bande; vuota se non c'e' nessuna delle due.
 */
export function logBand(dialHz: number, manualBand: string): string {
  return dialHz > 0 ? (bandOf(dialHz) ?? '') : manualBand;
}
