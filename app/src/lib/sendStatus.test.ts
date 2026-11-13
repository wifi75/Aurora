import { describe, expect, it } from 'vitest';
import { STRINGS } from '../i18n';
import { sendText } from './sendStatus';

// traduzione finta: la chiave e le variabili, per vedere cosa e' stato scelto
const t = (key: string, vars?: Record<string, string | number>) => (vars ? `${key}${JSON.stringify(vars)}` : key);

describe('sendText', () => {
  it('translates a ClubLog code with its values', () => {
    const st = { message: "Club Log ha negato l'accesso (Login failed); invio fermo", code: 'clublog_denied', params: { detail: 'Login failed' } };
    expect(sendText(st, t as never)).toBe('send.clublog_denied{"calls":"","detail":"Login failed","time":"","d":" (Login failed)"}');
  });

  it('uses the plain variant for a rejection without detail', () => {
    const st = { message: 'QSO con K1ABC rifiutato: dati non validi', code: 'clublog_rejected', params: { calls: 'K1ABC', detail: '' } };
    expect(sendText(st, t as never)).toContain('send.clublog_rejected.plain');
  });

  it('falls back to the raw text without a code or with an unknown one', () => {
    expect(sendText({ message: 'testo grezzo' }, t as never)).toBe('testo grezzo');
    expect(sendText({ message: 'testo grezzo', code: 'boh' }, t as never)).toBe('testo grezzo');
  });

  it('has every code in all six languages', () => {
    for (const [key, texts] of Object.entries(STRINGS)) {
      if (key.startsWith('send.')) expect((texts as readonly string[]).length, key).toBe(6);
    }
  });
});
