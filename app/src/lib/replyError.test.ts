import { describe, expect, it } from 'vitest';
import { ApiError } from '../api/client';
import { replyErrorMessage } from './replyError';

describe('replyErrorMessage', () => {
  it('explains that transmission is switched off in Aurora', () => {
    expect(replyErrorMessage(new ApiError(403, 'tx_disabled'))).toBe(
      'La trasmissione è disattivata in Aurora: nessuna risposta è stata inviata.',
    );
  });

  it('explains that this connection cannot transmit', () => {
    expect(replyErrorMessage(new ApiError(403, 'tx_not_allowed_on_transport'))).toBe(
      'Da questa connessione non si può trasmettere.',
    );
  });

  it('explains a decode that is gone and an unpaired device', () => {
    expect(replyErrorMessage(new ApiError(404, 'unknown_decode'))).toBe('Quel messaggio non è più disponibile.');
    expect(replyErrorMessage(new ApiError(401, 'unauthorized'))).toBe('Dispositivo non associato: associalo di nuovo.');
  });

  it('falls back to a generic message for anything else', () => {
    expect(replyErrorMessage(new TypeError('rete assente'))).toBe('Risposta non inviata. Controlla la connessione e riprova.');
    expect(replyErrorMessage(new ApiError(503, 'engine_unavailable'))).toBe(
      'Risposta non inviata. Controlla la connessione e riprova.',
    );
  });
});
