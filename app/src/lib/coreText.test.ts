import { describe, expect, it } from 'vitest';
import { STRINGS } from '../i18n';
import { coreText } from './coreText';

// traduzione finta: la chiave e le variabili, per vedere cosa e' stato riconosciuto
const t = (key: string, vars?: Record<string, string | number>) => (vars ? `${key}${JSON.stringify(vars)}` : key);

describe('coreText', () => {
  it('translates the known CAT messages, also inside the composed ones', () => {
    expect(coreText('Collegamento in corso', t as never)).toBe('core.connecting');
    const lost = "Collegamento perso: La radio non risponde: controlla accensione, cavo, porta, velocita' (baud) e, per le Yaesu, le impostazioni CAT della radio. Nuovo tentativo 2 di 8 tra 5 s.";
    expect(coreText(lost, t as never)).toBe('core.lost{"cause":"core.noAnswer","n":"2","max":"8","s":"5"}');
  });

  it('translates the evidence of the radio detection (issue #1)', () => {
    expect(coreText('il nome "IC-7300MK2 Serial Port A (CI-V)" contiene il modello', t as never)).toBe('core.ev.nameModel{"name":"IC-7300MK2 Serial Port A (CI-V)"}');
    expect(coreText("Errore sulla porta seriale: la connessione con la radio si e' interrotta. Nuovo tentativo tra 5 s.", t as never)).toBe('core.retry{"cause":"core.portError","s":"5"}');
  });

  it('translates the automation log and the audio errors', () => {
    expect(coreText('K1ABC: segnale -24 dB sotto il minimo di -20 dB', t as never)).toBe('core.au.weak{"call":"K1ABC","snr":"-24","min":"-20"}');
    expect(coreText('Rispondo a K1ABC: paese nuovo', t as never)).toBe('core.au.reply{"call":"K1ABC","why":"core.au.newDxcc"}');
    expect(coreText('K1ABC: tentativi esauriti senza QSO', t as never)).toBe('core.au.callWhy{"call":"K1ABC","why":"core.au.exhausted"}');
    expect(coreText('K1ABC: qualcosa di nuovo', t as never)).toBe('K1ABC: qualcosa di nuovo');
  });

  it('leaves an unknown message as it is', () => {
    expect(coreText('qualcosa di nuovo', t as never)).toBe('qualcosa di nuovo');
    expect(coreText(null, t as never)).toBe('');
  });

  it('has every message in six languages', () => {
    for (const [key, value] of Object.entries(STRINGS)) if (key.startsWith('core.')) expect(value).toHaveLength(6);
  });
});
