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

  it('translates the Cat4OM messages written by the core', () => {
    expect(coreText("Cat4OM ha rifiutato la chiave del gruppo: controlla la chiave e l'ora del PC", t as never)).toBe('core.cat4om.authFailed');
    expect(coreText('Cat4OM non raggiungibile su 127.0.0.1:17301: avvia Cat4OM e controlla la porta Control del gruppo; da un altro PC abilita la rete in Settings → SERVER (connessione rifiutata)', t as never)).toBe(
      'core.cat4om.noAnswer{"address":"127.0.0.1:17301","raw":"connessione rifiutata"}',
    );
    // la scoperta dalla porta di gestione
    expect(coreText('Gestione di Cat4OM non raggiungibile su 127.0.0.1:17300: avvia Log4OM e Cat4OM; da un altro PC abilita la rete in Settings → SERVER (connessione rifiutata)', t as never)).toBe(
      'core.cat4om.managementNoAnswer{"address":"127.0.0.1:17300","raw":"connessione rifiutata"}',
    );
    expect(coreText('Gruppo «group-9» non trovato in Cat4OM (presenti: group-1)', t as never)).toBe('core.cat4om.groupMissing{"group":"group-9","list":"group-1"}');
    expect(coreText("Il gruppo «group-1» di Cat4OM e' fermo: avvialo in Cat4OM", t as never)).toBe('core.cat4om.groupStopped{"group":"group-1"}');
    expect(coreText('Su 127.0.0.1:5001 non risponde la gestione di Cat4OM: controlla la porta di gestione (di base 17300)', t as never)).toBe('core.cat4om.notManagement{"address":"127.0.0.1:5001"}');
    expect(coreText('Su 127.0.0.1:17300 non risponde il Control di Cat4OM: controlla la porta del gruppo o lascia vuoto il campo «Controllo»', t as never)).toBe('core.cat4om.notControl{"address":"127.0.0.1:17300"}');
    expect(coreText("Nel campo «Controllo» c'e' la porta di gestione di Cat4OM: lascialo vuoto, Aurora trova da sola la porta del gruppo", t as never)).toBe('core.cat4om.management');
    expect(coreText('La gestione di Cat4OM chiede una password: scrivi la porta Control del gruppo nel campo «Controllo»', t as never)).toBe('core.cat4om.managementAuth');
    expect(coreText('Nessun gruppo in Cat4OM: crea in Cat4OM un gruppo con la radio', t as never)).toBe('core.cat4om.noGroups');
    // lo stato a radio collegata, come Decodium: resta com'e'
    expect(coreText('group-1/kenwood/ts-990 — master', t as never)).toBe('group-1/kenwood/ts-990 — master');
    expect(coreText('Radio Cat4OM «FT-991A» non collegata (disconnected)', t as never)).toBe('core.cat4om.radioOff{"radio":"FT-991A","state":"disconnected"}');
    expect(coreText("La radio «FT-991A» e' presa dall'emulazione hamlib di Cat4OM: spegni quel profilo in Cat4OM per comandarla da Aurora", t as never)).toBe(
      'core.cat4om.claimed{"radio":"FT-991A","by":"hamlib"}',
    );
    expect(coreText('Cat4OM ha rifiutato setMode: mode not supported (INVALID_MODE)', t as never)).toBe('core.cat4om.refused{"cmd":"setMode","raw":"mode not supported","code":"INVALID_MODE"}');
  });

  it('leaves an unknown message as it is', () => {
    expect(coreText('qualcosa di nuovo', t as never)).toBe('qualcosa di nuovo');
    expect(coreText(null, t as never)).toBe('');
  });

  it('has every message in six languages', () => {
    for (const [key, value] of Object.entries(STRINGS)) if (key.startsWith('core.')) expect(value).toHaveLength(6);
  });
});
