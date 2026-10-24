import { ApiError } from '../api/client';

/** Messaggio per chi tocca un decode e la risposta non parte. */
export function replyErrorMessage(error: unknown): string {
  if (error instanceof ApiError) {
    if (error.code === 'transmitting') return 'Non si cambia banda mentre si trasmette.';
    if (error.code === 'rig_error') return 'La radio non ha risposto: controlla OmniRig.';
    if (error.code === 'rig_unavailable') return 'Il comando della radio non è attivo (AURORA_RIG=omnirig).';
    if (error.code === 'tx_disabled') {
      return 'La trasmissione è disattivata in Aurora: nessuna risposta è stata inviata.';
    }
    if (error.status === 403) return 'Da questa connessione non si può trasmettere.';
    if (error.status === 404) return 'Quel messaggio non è più disponibile.';
    if (error.status === 401) return 'Dispositivo non associato: associalo di nuovo.';
  }
  return 'Risposta non inviata. Controlla la connessione e riprova.';
}
