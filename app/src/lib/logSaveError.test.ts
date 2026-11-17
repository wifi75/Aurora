import { describe, expect, it } from 'vitest';
import { ApiError } from '../api/client';
import { logBand, saveErrorText } from './logSaveError';

// traduttore minimo: restituisce la chiave e le variabili, cosi' si vede quale testo sceglie
const t = ((key: string, vars?: Record<string, string | number>) => `${key}${vars ? JSON.stringify(vars) : ''}`) as never;

describe('saveErrorText', () => {
  it('banda rifiutata dal log (400 bad_field band): il motivo e\' la banda sconosciuta', () => {
    expect(saveErrorText(new ApiError(400, 'bad_field', undefined, 'band'), t)).toBe('digi.log.saveNoBand');
  });
  it('un altro campo non valido lo nomina', () => {
    expect(saveErrorText(new ApiError(400, 'bad_field', undefined, 'call'), t)).toBe('digi.log.saveBadField{"field":"call"}');
  });
  it('altri errori: «QSO non salvato: <messaggio>», col testo del core se c\'e\'', () => {
    expect(saveErrorText(new ApiError(500, 'boom', undefined, 'disco pieno'), t)).toBe('digi.log.saveFail{"msg":"disco pieno"}');
    expect(saveErrorText(new TypeError('rete'), t)).toBe('digi.log.saveFail{"msg":"rete"}');
  });
});

describe('logBand', () => {
  it('con la frequenza della radio vale quella, anche se e\' stata scelta una banda a mano', () => {
    expect(logBand(28_400_000, '20m')).toBe('10m');
  });
  it('senza frequenza vale la banda scelta a mano, e vuota se manca anche quella', () => {
    expect(logBand(0, '15m')).toBe('15m');
    expect(logBand(0, '')).toBe('');
  });
});
