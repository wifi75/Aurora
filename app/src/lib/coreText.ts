import type { Key } from '../i18n';

type T = (key: Key, vars?: Record<string, string | number>) => string;

/**
 * I messaggi che il core scrive in italiano (stato e errori di CAT, Hamlib, OmniRig, PTT) riconosciuti e mostrati nella lingua scelta.
 * Un messaggio sconosciuto resta com'e'. I messaggi composti («Collegamento perso: <causa> Nuovo tentativo ...») si traducono a pezzi.
 */
export const EXACT: ReadonlyArray<[string, Key]> = [
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
  ["La radio e' spenta (PS0): accendila oppure spunta «Accendi» nella scheda Radio", 'core.ft991.off'],
  ['Collegamento con FT-991(A) Link perso: nuovo tentativo', 'core.ft991.lost'],
  ['servono il tuo nominativo e il tuo locatore (scheda Generali)', 'core.svc.needCall'],
  ['password di eQSL non impostata (o non leggibile da questo utente)', 'core.svc.eqslPassword'],
  ["Automazione spenta dall'operatore", 'core.au.offByUser'],
  ['STOP TX: automazione spenta', 'core.au.stop'],
  ['Nominativo non leggibile nel messaggio', 'core.au.noCall'],
  ['paese nuovo', 'core.au.newDxcc'],
  ['paese nuovo su questa banda', 'core.au.newDxccBand'],
  ['zona nuova', 'core.au.newZone'],
  ['griglia nuova', 'core.au.newGrid'],
  ['CQ di interesse', 'core.au.cqOfInterest'],
  ['decodifica incerta (segnata con «?» dal motore)', 'core.au.uncertain'],
  ['il nominativo non ha la forma di un nominativo', 'core.au.badShape'],
  ['segnale molto debole di una stazione mai sentita prima: possibile falso decode', 'core.au.ghost'],
  ['tentativi esauriti senza QSO', 'core.au.exhausted'],
  ['paese nuovo in questo modo', 'core.au.newMode'],
  ['continente sconosciuto', 'core.au.contUnknown'],
  ["Di nuovo nella fascia oraria: l'automazione riprende", 'core.au.inWindow'],
];

/** Frasi con valori dentro: chiave e nomi delle variabili nell'ordine dei gruppi; `inner` sono i gruppi da tradurre a loro volta. */
const PATTERNS: ReadonlyArray<{ re: RegExp; key: Key; vars: string[]; inner?: string[] }> = [
  // FT-991(A) Link: il programma non risponde (con l'errore del socket tra parentesi, se c'e')
  { re: /^FT-991\(A\) Link non risponde su (\S+): in FT991Link apri COM Port e premi Start UDP(?: \((.*)\))?$/s, key: 'core.ft991.noAnswer', vars: ['address', 'raw'] },
  // il modo arriva da `format!` nel core: non e' una frase intera da controllare
  { re: /^Automazione accesa, modo reale$/s, key: 'core.au.onLive', vars: [] },
  { re: /^Automazione accesa, modo in prova a vuoto$/s, key: 'core.au.onDry', vars: [] },
  { re: /^Nessun dispositivo collegato da (\d+) s: automazione spenta$/s, key: 'core.au.deadMan', vars: ['s'] },
  { re: /^Tra (\d+) CQ interessanti scelto (\S+) \(altri: (.*)\)$/s, key: 'core.au.chose', vars: ['n', 'call', 'others'] },
  { re: /^(\S+): segnale (-?\d+) dB sotto il minimo di (-?\d+) dB$/s, key: 'core.au.weak', vars: ['call', 'snr', 'min'] },
  { re: /^(\S+): QSO gia' in corso con (\S+)$/s, key: 'core.au.busy', vars: ['call', 'busy'] },
  { re: /^(\S+) e' nell'elenco delle esclusioni$/s, key: 'core.au.excluded', vars: ['call'] },
  { re: /^Raggiunte (\d+) risposte di fila senza QSO: automazione spenta$/s, key: 'core.au.maxRow', vars: ['n'] },
  { re: /^Raggiunto il tetto di (\d+) risposte al giorno$/s, key: 'core.au.maxDay', vars: ['n'] },
  { re: /^Raggiunto il limite di (\d+) QSO all'ora$/s, key: 'core.au.maxHour', vars: ['n'] },
  { re: /^Fuori dalla fascia oraria (\S+) \(ora locale\): automazione in attesa$/s, key: 'core.au.outWindow', vars: ['range'] },
  { re: /^Nessun CQ a cui rispondere da (\d+) min: chiamerei CQ \(prova a vuoto\)$/s, key: 'core.au.wouldCallCq', vars: ['n'] },
  { re: /^Nessun CQ a cui rispondere da (\d+) min: chiamo CQ$/s, key: 'core.au.callCq', vars: ['n'] },
  { re: /^(\S+): (.+) e' tra i paesi esclusi$/s, key: 'core.au.countryExcluded', vars: ['call', 'country'] },
  { re: /^(\S+): continente (\S+) non tra quelli scelti$/s, key: 'core.au.contOff', vars: ['call', 'cont'] },
  { re: /^Rispondo a (\S+): (.*)$/s, key: 'core.au.reply', vars: ['call', 'why'], inner: ['why'] },
  { re: /^Risponderei a (\S+): (.*) \(prova a vuoto\)$/s, key: 'core.au.wouldReply', vars: ['call', 'why'], inner: ['why'] },
  { re: /^nella lista dei desiderati \((.*)\)$/s, key: 'core.au.wanted', vars: ['why'] },
  { re: /^CQ (\S+): non e' per il tuo continente \((\S+)\)$/s, key: 'core.au.notContinent', vars: ['to', 'own'] },
  { re: /^CQ DX dal tuo stesso continente \((\S+)\)$/s, key: 'core.au.sameContinent', vars: ['own'] },
  { re: /^ingresso audio non trovato \((.*)\); disponibili: (.*)$/s, key: 'core.audio.notFound', vars: ['name', 'list'] },
  { re: /^configurazione audio non disponibile: (.*)$/s, key: 'core.audio.noConfig', vars: ['raw'] },
  { re: /^ingresso audio non aperto \(occupato da un'altra applicazione\?\): (.*)$/s, key: 'core.audio.busy', vars: ['raw'] },
  { re: /^QSO con (.+) scartato: in coda da piu' di sette giorni$/s, key: 'core.svc.dropped', vars: ['calls'] },
  { re: /^QSO con (.+?) inviato(?: \((.*)\))?$/s, key: 'core.svc.sent', vars: ['calls', 'detail'] },
  { re: /^QSO con (.+?) gia' presente(?: \((.*)\))?$/s, key: 'core.svc.dup', vars: ['calls', 'detail'] },
  { re: /^QSO con (.+?) rifiutato: (.*)$/s, key: 'core.svc.refused', vars: ['calls', 'why'] },
  { re: /^(.*): nuovo tentativo alle (\d\d:\d\d) UTC$/s, key: 'core.svc.retryAt', vars: ['why', 'at'], inner: ['why'] },
  { re: /^spot inviato: (\S+) su (\S+) kHz$/s, key: 'core.svc.spot', vars: ['call', 'khz'] },
  { re: /^(\d+) spot in (\d+) pacchetti$/s, key: 'core.svc.psk', vars: ['n', 'p'] },
  { re: /^QSO inviato a (.+)$/s, key: 'core.svc.tcp', vars: ['to'] },
  { re: /^invio interrotto: (.*)$/s, key: 'core.svc.aborted', vars: ['why'] },
  // «CALL: motivo» per ultimo: il motivo si traduce se e' noto
  { re: /^(\S+): (.+)$/s, key: 'core.au.callWhy', vars: ['call', 'why'], inner: ['why'] },
];

const LOST = /^Collegamento perso: (.*) Nuovo tentativo (\d+) di (\d+) tra (\d+) s\.$/s;
const RETRY = /^(.*) Nuovo tentativo tra (\d+) s\.$/s;
const STOPPED = /^rigctld si e' fermato: controlla modello, porta e velocita'\. \((.*)\)$/s;
const PORT_USED = /^la porta (\d+) e' gia' usata da (?:il programma «(.+)»|un altro programma \(forse Aurora e' gia' aperta\)): chiudilo o scegli un'altra porta in Impostazioni > Accesso$/s;

const EVIDENCE: ReadonlyArray<[RegExp, Key]> = [
  [/^il nome "(.*)" contiene il modello$/s, 'core.ev.nameModel'],
  [/^il nome "(.*)" e' quello di un'interfaccia radio$/s, 'core.ev.nameInterface'],
  [/^porta seriale (.*)$/s, 'core.ev.serial'],
  [/^Adattatore USB-seriale \((.*)\): il modello va scelto a mano$/s, 'core.ev.adapter'],
];
const USB_ID = /^identita' USB ([0-9A-F]{4}:[0-9A-F]{4})(, porta CAT \(Enhanced\))?$/s;

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
  for (const [re, key] of EVIDENCE) {
    const e = re.exec(m);
    if (e) return t(key, { name: e[1] ?? '' });
  }
  r = USB_ID.exec(m);
  if (r) return t(r[2] ? 'core.ev.usbCat' : 'core.ev.usb', { id: r[1] ?? '' });
  r = PORT_USED.exec(m);
  if (r) return t(r[2] ? 'core.portUsedBy' : 'core.portUsed', { port: r[1] ?? '', who: r[2] ?? '' });
  for (const p of PATTERNS) {
    const g = p.re.exec(m);
    if (!g) continue;
    const vars: Record<string, string> = {};
    p.vars.forEach((name, i) => {
      const v = g[i + 1] ?? '';
      vars[name] = p.inner?.includes(name) ? coreText(v, t) : v;
    });
    // «CALL: motivo» con un motivo sconosciuto: si lascia tutto com'era
    if (p.key === 'core.au.callWhy' && vars.why === (g[2] ?? '')) return m;
    // il dettaglio tra parentesi c'e' solo se il core l'ha scritto
    if ('detail' in vars && !vars.detail) return t(`${p.key}.plain` as Key, vars);
    return t(p.key, vars);
  }
  return m;
}
