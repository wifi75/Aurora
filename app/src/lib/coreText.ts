import type { Key } from '../i18n';

type T = (key: Key, vars?: Record<string, string | number>) => string;

/**
 * I messaggi che il core scrive in italiano (stato e errori di CAT, Hamlib, OmniRig, PTT) riconosciuti e mostrati nella lingua scelta.
 * Un messaggio sconosciuto resta com'e'. I messaggi composti («Collegamento perso: <causa> Nuovo tentativo ...») si traducono a pezzi.
 */
const EXACT: ReadonlyArray<[string, Key]> = [
  ["Manca rigctld (Hamlib): mettilo accanto ad Aurora o indica il percorso nella variabile AURORA_RIGCTLD.", 'core.rigctld'],
  ["La porta seriale e' occupata da un altro programma: chiudilo (o scegli un'altra porta) e riprova.", 'core.portBusy'],
  ["La porta seriale non esiste: controlla il numero (Windows puo' rinumerarla) e che la radio sia accesa e collegata.", 'core.portMissing'],
  ["La radio non risponde: controlla accensione, cavo, porta, velocita' (baud) e, per le Yaesu, le impostazioni CAT della radio.", 'core.noAnswer'],
  ["Errore sulla porta seriale: la connessione con la radio si e' interrotta.", 'core.portError'],
  ['La radio non supporta questo comando.', 'core.unsupported'],
  ['La radio ha rifiutato il comando.', 'core.refused'],
  ['Collegamento in corso', 'core.connecting'],
  ['Collegamento a OmniRig', 'core.connectingOmni'],
  ['stato non disponibile', 'core.noStatus'],
  ['thread del CAT non avviato', 'core.noThread'],
  ['Chiusa, ma il PTT potrebbe essere rimasto acceso: controlla la radio.', 'core.pttMaybeOn'],
  ['radio non in linea', 'core.omniOffline'],
  ["OmniRig non e' installato su questo PC: in Impostazioni > Radio scegli Hamlib (gia' incluso in Aurora) oppure installa OmniRig", 'core.omniMissing'],
  [
    "OmniRig non risponde: apri OmniRig e controlla che Rig1 abbia la porta e il modello giusti e la radio accesa (stato «On-line»), oppure in Impostazioni > Radio scegli Hamlib (gia' incluso in Aurora)",
    'core.omniNoAnswer',
  ],
  ['con OmniRig Aurora comanda solo la frequenza della radio', 'core.omniFreqOnly'],
  [
    "con OmniRig le impostazioni della radio (preamplificatore, AGC, NB, VFO...) non si cambiano da qui: scegli il CAT Hamlib in Impostazioni → Radio",
    'core.omniNoExtras',
  ],
  ['nessuna risposta', 'core.noReply'],
];

const LOST = /^Collegamento perso: (.*) Nuovo tentativo (\d+) di (\d+) tra (\d+) s\.$/s;
const RETRY = /^(.*) Nuovo tentativo tra (\d+) s\.$/s;
const STOPPED = /^rigctld si e' fermato: controlla modello, porta e velocita'\. \((.*)\)$/s;
const PORT_USED = /^la porta (\d+) e' gia' usata da (?:il programma «(.+)»|un altro programma \(forse Aurora e' gia' aperta\)): chiudilo o scegli un'altra porta in Impostazioni > Accesso$/s;

export function coreText(message: string | null | undefined, t: T): string {
  const m = (message ?? '').trim();
  if (m === '') return '';
  const hit = EXACT.find(([it]) => it === m);
  if (hit) return t(hit[1]);
  let r = LOST.exec(m);
  if (r) return t('core.lost', { cause: coreText(r[1], t), n: r[2] ?? '', max: r[3] ?? '', s: r[4] ?? '' });
  r = RETRY.exec(m);
  if (r) return t('core.retry', { cause: coreText(r[1], t), s: r[2] ?? '' });
  r = STOPPED.exec(m);
  if (r) return t('core.rigctldStopped', { raw: r[1] ?? '' });
  r = PORT_USED.exec(m);
  if (r) return t(r[2] ? 'core.portUsedBy' : 'core.portUsed', { port: r[1] ?? '', who: r[2] ?? '' });
  return m;
}
