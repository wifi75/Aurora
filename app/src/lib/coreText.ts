import type { Key } from '../i18n';

type T = (key: Key, vars?: Record<string, string | number>) => string;

/**
 * I messaggi che il core scrive in italiano (stato e errori di CAT, Hamlib, OmniRig, PTT) riconosciuti e mostrati nella lingua scelta.
 * Un messaggio sconosciuto resta com'e'. I messaggi composti («Collegamento perso: <causa> Nuovo tentativo ...») si traducono a pezzi.
 */
export const EXACT: ReadonlyArray<[string, Key]> = [
  ["Manca rigctld (Hamlib): probabilmente l'antivirus l'ha tolto. Su Windows premi «Ripristina Hamlib»; altrimenti mettilo accanto ad Aurora o indica il percorso nella variabile AURORA_RIGCTLD.", 'core.rigctld'],
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
  ["OmniRig non risponde: la radio e' accesa e in linea in OmniRig?", 'core.omniNotReady'],
  ["OmniRig non e' installato su questo PC: in Impostazioni › Stazione › Radio scegli Hamlib (gia' incluso in Aurora) oppure installa OmniRig", 'core.omniMissing'],
  [
    "OmniRig non risponde: apri OmniRig e controlla che Rig1 abbia la porta e il modello giusti e la radio accesa (stato «On-line»), oppure in Impostazioni › Stazione › Radio scegli Hamlib (gia' incluso in Aurora)",
    'core.omniNoAnswer',
  ],
  ['con OmniRig Aurora comanda solo la frequenza della radio', 'core.omniFreqOnly'],
  [
    "con OmniRig le impostazioni della radio (preamplificatore, AGC, NB, VFO...) non si cambiano da qui: scegli il CAT Hamlib in Impostazioni › Stazione › Radio",
    'core.omniNoExtras',
  ],
  ['nessuna risposta', 'core.noReply'],
  ["La radio e' spenta (PS0): accendila oppure spunta «Accendi» nella scheda Radio", 'core.ft991.off'],
  ['Collegamento con FT-991(A) Link perso: nuovo tentativo', 'core.ft991.lost'],
  ['radio Cat4OM non collegata', 'core.cat4om.notLinked'],
  ['Nessuna radio nel gruppo Cat4OM: aggiungila in Cat4OM', 'core.cat4om.noRadios'],
  ['Cat4OM chiede la chiave del gruppo: scrivila in Impostazioni › Stazione › Radio', 'core.cat4om.authRequired'],
  ["Cat4OM ha rifiutato la chiave del gruppo: controlla la chiave e l'ora del PC", 'core.cat4om.authFailed'],
  ['Versione del protocollo Cat4OM non compatibile con Aurora (1.0.0)', 'core.cat4om.version'],
  ["Nel campo «Controllo» c'e' la porta di gestione di Cat4OM: lascialo vuoto, Aurora trova da sola la porta del gruppo", 'core.cat4om.management'],
  ['La gestione di Cat4OM chiede una password: scrivi la porta Control del gruppo nel campo «Controllo»', 'core.cat4om.managementAuth'],
  ['Nessun gruppo in Cat4OM: crea in Cat4OM un gruppo con la radio', 'core.cat4om.noGroups'],
  // audio in rete di Cat4OM (`cat4om_audio.rs`, `audio_control.rs`)
  ["Questa versione di Cat4OM non ha l'audio in rete: aggiorna Cat4OM o usa una scheda audio", 'core.cat4om.audio.noFeature'],
  ["Cat4OM: per trasmettere l'audio in rete Aurora deve avere il comando della radio, che ora e' di un altro programma", 'core.cat4om.audio.notMaster'],
  ["Cat4OM: un altro programma sta gia' mandando l'audio di trasmissione a questa radio", 'core.cat4om.audio.txInUse'],
  ['Cat4OM: audio in rete non configurato per questa radio (scegli i dispositivi audio della radio in Cat4OM)', 'core.cat4om.audio.unavailable'],
  ["Cat4OM ha aperto l'audio in rete in un formato che Aurora non conosce", 'core.cat4om.audio.badGrant'],
  ['ingresso Cat4OM: scegli prima Cat4OM in Impostazioni › Stazione › Radio', 'core.cat4om.audio.needsCatIn'],
  ['uscita Cat4OM: scegli prima Cat4OM in Impostazioni › Stazione › Radio', 'core.cat4om.audio.needsCatOut'],
  ['Accensione del ricevitore RX2', 'core.tci.rx2Wait'],
  ["La SDR non ha acceso il ricevitore RX2 (rx_enable): controlla che la SDR sia accesa e che abbia RX2, poi ricollega", 'core.tci.rx2Failed'],
  ['La SDR non ha ancora detto la potenza (drive): cambiala una volta dalla SDR', 'core.tci.noDrive'],
  ['servono il tuo nominativo e il tuo locatore (scheda Generali)', 'core.svc.needCall'],
  ['password di eQSL non impostata (o non leggibile da questo utente)', 'core.svc.eqslPassword'],
  ["Automazione spenta dall'operatore", 'core.au.offByUser'],
  ['STOP TX: automazione spenta', 'core.au.stop'],
  ['Nominativo non leggibile nel messaggio', 'core.au.noCall'],
  ["nascosta dai filtri di Attivita' di banda", 'core.au.hiddenByFilters'],
  ['Abilita Tx acceso dalle automazioni (si rispegne quando le automazioni si fermano)', 'core.au.txOn'],
  ["Abilita Tx rimesso spento, com'era prima delle automazioni", 'core.au.txBack'],
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
  // porta seriale tenuta da un altro programma (`rig_control::busy_port`): OmniRig ha il suo consiglio, il nome noto, nessun nome
  { re: /^La porta (\S+) e' occupata da OmniRig: chiudi OmniRig oppure scegli OmniRig come sistema di controllo in Impostazioni › Stazione › Radio$/s, key: 'core.portHeldOmni', vars: ['port'] },
  { re: /^La porta (\S+) e' occupata da un altro programma \(WSJT-X, JTDX, il log\.\.\.\): chiudilo e premi Collega$/s, key: 'core.portHeld', vars: ['port'] },
  { re: /^La porta (\S+) e' occupata da (.+?): chiudi \2 e premi Collega$/s, key: 'core.portHeldBy', vars: ['port', 'who'] },
  // la porta del PTT su seriale (RTS/DTR) si apre all'avvio: il consiglio e' riavviare Aurora (`busy_port::ptt_busy_message`)
  { re: /^La porta (\S+) e' occupata da un altro programma \(WSJT-X, JTDX, il log\.\.\.\): chiudilo e riavvia Aurora$/s, key: 'core.pttPortHeld', vars: ['port'] },
  { re: /^La porta (\S+) e' occupata da (.+?): chiudi \2 e riavvia Aurora$/s, key: 'core.pttPortHeldBy', vars: ['port', 'who'] },
  // FT-991(A) Link: il programma non risponde (con l'errore del socket tra parentesi, se c'e')
  { re: /^FT-991\(A\) Link non risponde su (\S+): in FT991Link apri COM Port e premi Start UDP(?: \((.*)\))?$/s, key: 'core.ft991.noAnswer', vars: ['address', 'raw'] },
  // Cat4OM: servizio non raggiungibile, porta sbagliata, radio mancante, scollegata o presa dall'emulazione, collegamento perso, rifiuti
  {
    re: /^Cat4OM non raggiungibile su (\S+): avvia Cat4OM e controlla la porta Control del gruppo; da un altro PC abilita la rete in Settings → SERVER \((.*)\)$/s,
    key: 'core.cat4om.noAnswer',
    vars: ['address', 'raw'],
  },
  { re: /^Su (\S+) non risponde il Control di Cat4OM: controlla la porta del gruppo o lascia vuoto il campo «Controllo»$/s, key: 'core.cat4om.notControl', vars: ['address'] },
  // la scoperta dalla porta di gestione (17300): servizio spento, porta sbagliata, gruppo mancante o fermo
  {
    re: /^Gestione di Cat4OM non raggiungibile su (\S+): avvia Log4OM e Cat4OM; da un altro PC abilita la rete in Settings → SERVER \((.*)\)$/s,
    key: 'core.cat4om.managementNoAnswer',
    vars: ['address', 'raw'],
  },
  { re: /^Su (\S+) non risponde la gestione di Cat4OM: controlla la porta di gestione \(di base 17300\)$/s, key: 'core.cat4om.notManagement', vars: ['address'] },
  { re: /^Gruppo «(.*)» non trovato in Cat4OM \(presenti: (.*)\)$/s, key: 'core.cat4om.groupMissing', vars: ['group', 'list'] },
  { re: /^Il gruppo «(.*)» di Cat4OM e' fermo: avvialo in Cat4OM$/s, key: 'core.cat4om.groupStopped', vars: ['group'] },
  { re: /^Radio «(.*)» non trovata nel gruppo Cat4OM \(presenti: (.*)\)$/s, key: 'core.cat4om.radioMissing', vars: ['radio', 'list'] },
  { re: /^Radio Cat4OM «(.*)» non collegata \((\S+)\)$/s, key: 'core.cat4om.radioOff', vars: ['radio', 'state'] },
  {
    re: /^La radio «(.*)» e' presa dall'emulazione (\S+) di Cat4OM: spegni quel profilo in Cat4OM per comandarla da Aurora$/s,
    key: 'core.cat4om.claimed',
    vars: ['radio', 'by'],
  },
  { re: /^Collegamento con Cat4OM perso \((.*)\): nuovo tentativo$/s, key: 'core.cat4om.lost', vars: ['raw'] },
  { re: /^Cat4OM: un'altra radio o un altro programma sta trasmettendo \((\S+) rifiutato\)$/s, key: 'core.cat4om.txBusy', vars: ['cmd'] },
  { re: /^Cat4OM non risponde a (\S+) \(nessuna risposta in (\d+) s\)$/s, key: 'core.cat4om.timeout', vars: ['cmd', 's'] },
  { re: /^Cat4OM ha rifiutato (\S+): (.*) \((\S+)\)$/s, key: 'core.cat4om.refused', vars: ['cmd', 'raw', 'code'] },
  { re: /^Cat4OM non accetta l'audio a (\d+) Hz per questa radio$/s, key: 'core.cat4om.audio.rate', vars: ['rate'] },
  { re: /^Cat4OM non riesce ad aprire il dispositivo audio della radio: controlla che sia collegato \((\S+)\)$/s, key: 'core.cat4om.audio.device', vars: ['code'] },
  { re: /^Cat4OM ha chiuso l'audio in rete: (.*)$/s, key: 'core.cat4om.audio.stopped', vars: ['raw'] },
  // TCI: comando senza eco (nome del comando del protocollo) e frequenza rifiutata
  { re: /^La radio TCI non ha confermato il comando (\S+) \(nessuna eco dal server, ripetuto una volta\)$/s, key: 'core.tci.noEcho', vars: ['cmd'] },
  { re: /^La radio TCI ha rifiutato la frequenza (\d+) Hz: e' rimasta su (\d+) Hz$/s, key: 'core.tci.freqRefused', vars: ['asked', 'got'] },
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
const PORT_USED = /^la porta (\d+) e' gia' usata da (?:il programma «(.+)»|un altro programma \(forse Aurora e' gia' aperta\)): chiudilo o scegli un'altra porta in Impostazioni › Accesso e assistenza › Smartphone$/s;

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
