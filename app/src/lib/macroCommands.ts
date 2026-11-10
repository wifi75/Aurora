/** L'elenco dei comandi delle macro che Aurora esegue, per la finestra di modifica: gruppo, comando e spiegazione nelle sei lingue (it, en, de, fr, es, pt). */

export type CommandGroup = 'text' | 'program' | 'frequency' | 'mode';

export interface MacroCommand {
  group: CommandGroup;
  /** Il comando come si scrive nello script. */
  tag: string;
  /** La spiegazione: [it, en, de, fr, es, pt]. */
  desc: readonly [string, string, string, string, string, string];
}

const c = (group: CommandGroup, tag: string, ...desc: [string, string, string, string, string, string]): MacroCommand => ({ group, tag, desc });

export const MACRO_COMMANDS: readonly MacroCommand[] = [
  c('text', '<MYCALL>', 'il tuo nominativo', 'your callsign', 'dein Rufzeichen', 'ton indicatif', 'tu indicativo', 'o teu indicativo'),
  c('text', '<MYMAINCALL>', 'il tuo nominativo senza /P, /M...', 'your callsign without /P, /M...', 'dein Rufzeichen ohne /P, /M ...', 'ton indicatif sans /P, /M...', 'tu indicativo sin /P, /M...', 'o teu indicativo sem /P, /M...'),
  c('text', '<MYNAME>', 'il tuo nome', 'your name', 'dein Name', 'ton prénom', 'tu nombre', 'o teu nome'),
  c('text', '<MYQTH>', 'il tuo QTH', 'your QTH', 'dein QTH', 'ton QTH', 'tu QTH', 'o teu QTH'),
  c('text', '<MYWWLOC>', 'il tuo locatore', 'your locator', 'dein Locator', 'ton locator', 'tu locator', 'o teu locator'),
  c('text', '<CALL>', 'nominativo del corrispondente', 'callsign of the other station', 'Rufzeichen der Gegenstation', 'indicatif du correspondant', 'indicativo del corresponsal', 'indicativo do correspondente'),
  c('text', '<NAME>', 'nome del corrispondente', 'name of the other operator', 'Name der Gegenstation', 'prénom du correspondant', 'nombre del corresponsal', 'nome do correspondente'),
  c('text', '<QTH>', 'QTH del corrispondente', 'QTH of the other station', 'QTH der Gegenstation', 'QTH du correspondant', 'QTH del corresponsal', 'QTH do correspondente'),
  c('text', '<WWLOC>', 'locatore del corrispondente', 'locator of the other station', 'Locator der Gegenstation', 'locator du correspondant', 'locator del corresponsal', 'locator do correspondente'),
  c('text', '<RSTS>', 'rapporto da mandare', 'RST to send', 'zu sendender Rapport', 'rapport à envoyer', 'informe a enviar', 'relatório a enviar'),
  c('text', '<RSTR>', 'rapporto ricevuto', 'RST received', 'empfangener Rapport', 'rapport reçu', 'informe recibido', 'relatório recebido'),
  c('text', '<NOTES>', 'campo note', 'notes field', 'Notizfeld', 'champ notes', 'campo de notas', 'campo de notas'),
  c('text', '<NRS>', 'scambio da mandare (contest)', 'exchange to send (contest)', 'zu sendender Austausch (Contest)', 'échange à envoyer (concours)', 'intercambio a enviar (concurso)', 'troca a enviar (concurso)'),
  c('text', '<NRR>', 'scambio ricevuto (contest)', 'exchange received (contest)', 'empfangener Austausch (Contest)', 'échange reçu (concours)', 'intercambio recibido (concurso)', 'troca recebida (concurso)'),
  c('text', '<DATE>', 'data di oggi (UTC)', 'today’s date (UTC)', 'heutiges Datum (UTC)', 'date du jour (UTC)', 'fecha de hoy (UTC)', 'data de hoje (UTC)'),
  c('text', '<TIME>', 'ora UTC', 'UTC time', 'UTC-Zeit', 'heure UTC', 'hora UTC', 'hora UTC'),
  c('text', '<LOCALTIME>', 'ora locale', 'local time', 'Ortszeit', 'heure locale', 'hora local', 'hora local'),
  c('text', '<STARTTIME>', 'ora di inizio del QSO', 'QSO start time', 'Beginn des QSO', 'heure de début du QSO', 'hora de inicio del QSO', 'hora de início do QSO'),
  c('text', '<CTIME>', 'ora UTC in formato HHMM', 'UTC time as HHMM', 'UTC-Zeit als HHMM', 'heure UTC en HHMM', 'hora UTC como HHMM', 'hora UTC como HHMM'),
  c('text', '<GA>', 'GM, GA o GE secondo l’ora', 'GM, GA or GE by the time', 'GM, GA oder GE nach der Zeit', 'GM, GA ou GE selon l’heure', 'GM, GA o GE según la hora', 'GM, GA ou GE conforme a hora'),
  c('text', '<GAL>', 'come <GA>, frase lunga', 'like <GA>, long phrase', 'wie <GA>, langer Satz', 'comme <GA>, phrase longue', 'como <GA>, frase larga', 'como <GA>, frase longa'),
  c('text', '<KHZ>', 'frequenza della radio in kHz (<KHZ:n> con n decimali)', 'radio frequency in kHz (<KHZ:n> for n decimals)', 'Funkfrequenz in kHz (<KHZ:n> mit n Dezimalen)', 'fréquence de la radio en kHz (<KHZ:n> avec n décimales)', 'frecuencia de la radio en kHz (<KHZ:n> con n decimales)', 'frequência do rádio em kHz (<KHZ:n> com n decimais)'),
  c('text', '<MHZ>', 'frequenza della radio in MHz (<MHZ:n>)', 'radio frequency in MHz (<MHZ:n>)', 'Funkfrequenz in MHz (<MHZ:n>)', 'fréquence de la radio en MHz (<MHZ:n>)', 'frecuencia de la radio en MHz (<MHZ:n>)', 'frequência do rádio em MHz (<MHZ:n>)'),
  c('text', '<BAND>', 'banda in uso', 'band in use', 'verwendetes Band', 'bande utilisée', 'banda en uso', 'banda em uso'),
  c('text', '<MODE>', 'modo in uso', 'mode in use', 'verwendeter Modus', 'mode utilisé', 'modo en uso', 'modo em uso'),
  c('text', '<QSONR>', 'numero del QSO', 'QSO number', 'QSO-Nummer', 'numéro du QSO', 'número del QSO', 'número do QSO'),
  c('text', '<CR>', 'ritorno a capo (CR)', 'carriage return (CR)', 'Wagenrücklauf (CR)', 'retour chariot (CR)', 'retorno de carro (CR)', 'retorno de carro (CR)'),
  c('text', '<LF>', 'nuova riga (LF)', 'line feed (LF)', 'Zeilenvorschub (LF)', 'saut de ligne (LF)', 'salto de línea (LF)', 'avanço de linha (LF)'),
  c('text', '<CRLF>', 'a capo completo (CR + LF)', 'full new line (CR + LF)', 'voller Zeilenumbruch (CR + LF)', 'nouvelle ligne complète (CR + LF)', 'nueva línea completa (CR + LF)', 'nova linha completa (CR + LF)'),
  c('text', '<TAB>', 'tabulazione', 'tab character', 'Tabulator', 'tabulation', 'tabulación', 'tabulação'),
  c('text', '<VER>', 'nome del programma', 'program name', 'Programmname', 'nom du programme', 'nombre del programa', 'nome do programa'),
  c('program', '<TX>', 'passa subito in trasmissione', 'switch to transmit at once', 'sofort auf Senden schalten', 'passer tout de suite en émission', 'pasar a transmitir de inmediato', 'passar logo a transmitir'),
  c('program', '<TXE>', 'trasmette quando la portante in ricezione cade', 'transmit when the received carrier drops', 'sendet, wenn der Empfangsträger fällt', 'émet quand la porteuse reçue tombe', 'transmite cuando cae la portadora recibida', 'transmite quando a portadora recebida cai'),
  c('program', '<TXTOGGLE>', 'alterna fra trasmissione e ricezione', 'toggle between transmit and receive', 'zwischen Senden und Empfang umschalten', 'alterne entre émission et réception', 'alterna entre transmisión y recepción', 'alterna entre transmissão e receção'),
  c('program', '<RX>', 'torna in ricezione quando il testo è trasmesso', 'back to receive when the text is sent', 'zurück zum Empfang, wenn der Text gesendet ist', 'retour en réception quand le texte est émis', 'vuelve a recepción cuando se transmite el texto', 'volta à receção quando o texto é transmitido'),
  c('program', '<RXANDCLEAR>', 'come <RX> e pulisce la finestra di trasmissione', 'like <RX> and clears the transmit window', 'wie <RX> und leert das Sendefenster', 'comme <RX> et efface la fenêtre d’émission', 'como <RX> y limpia la ventana de transmisión', 'como <RX> e limpa a janela de transmissão'),
  c('program', '<RXC>', 'uguale a <RXANDCLEAR>', 'same as <RXANDCLEAR>', 'wie <RXANDCLEAR>', 'identique à <RXANDCLEAR>', 'igual que <RXANDCLEAR>', 'igual a <RXANDCLEAR>'),
  c('program', '<RXE>', 'torna in ricezione a fine testo', 'back to receive when all text is sent', 'zurück zum Empfang nach dem ganzen Text', 'retour en réception à la fin du texte', 'vuelve a recepción al terminar el texto', 'volta à receção no fim do texto'),
  c('program', '<CLEARRXWINDOW>', 'pulisce la finestra di ricezione', 'clears the receive window', 'leert das Empfangsfenster', 'efface la fenêtre de réception', 'limpia la ventana de recepción', 'limpa a janela de receção'),
  c('program', '<CLEARTXWINDOW>', 'pulisce la finestra di trasmissione', 'clears the transmit window', 'leert das Sendefenster', 'efface la fenêtre d’émission', 'limpia la ventana de transmisión', 'limpa a janela de transmissão'),
  c('program', '<CLEARWINDOW>', 'pulisce la finestra in uso', 'clears the current window', 'leert das aktuelle Fenster', 'efface la fenêtre en cours', 'limpia la ventana actual', 'limpa a janela atual'),
  c('program', '<CLEARQSO>', 'azzera i dati del QSO', 'clears the QSO data', 'löscht die QSO-Daten', 'efface les données du QSO', 'borra los datos del QSO', 'apaga os dados do QSO'),
  c('program', '<FIXSTARTTIME>', 'fissa adesso come ora di inizio del QSO', 'fixes now as the QSO start time', 'setzt jetzt als Beginn des QSO', 'fixe maintenant comme début du QSO', 'fija ahora como inicio del QSO', 'fixa agora como início do QSO'),
  c('program', '<SAVEQSO>', 'salva il QSO a log (va prima dei comandi di pulizia)', 'saves the QSO to the log (put it before any clear command)', 'speichert das QSO im Log (vor den Löschbefehlen)', 'enregistre le QSO au journal (avant les commandes d’effacement)', 'guarda el QSO en el log (antes de los comandos de limpieza)', 'guarda o QSO no log (antes dos comandos de limpeza)'),
  c('program', '<AUTOCQ>', 'avvia o ferma la chiamata CQ automatica', 'starts or stops the automatic CQ', 'startet oder stoppt den automatischen CQ-Ruf', 'démarre ou arrête l’appel CQ automatique', 'inicia o detiene la llamada CQ automática', 'inicia ou pára a chamada CQ automática'),
  c('program', '<TUNE>', 'manda un tono per accordare (<TUNE:n> per n secondi)', 'sends a tuning tone (<TUNE:n> for n seconds)', 'sendet einen Abstimmton (<TUNE:n> für n Sekunden)', 'envoie une porteuse d’accord (<TUNE:n> pendant n secondes)', 'envía un tono de sintonía (<TUNE:n> durante n segundos)', 'envia um tom de sintonia (<TUNE:n> durante n segundos)'),
  c('program', '<BEEP:ON>', 'suona un avviso', 'sounds a beep', 'gibt einen Signalton aus', 'émet un bip', 'emite un pitido', 'emite um aviso sonoro'),
  c('program', '<CURSOR:TX>', 'porta il cursore nella casella di trasmissione', 'puts the cursor in the transmit box', 'setzt den Cursor ins Sendefeld', 'place le curseur dans la case d’émission', 'pone el cursor en la casilla de transmisión', 'põe o cursor na caixa de transmissão'),
  c('program', '<MACROS:file>', 'carica un altro insieme di macro', 'loads another macro set', 'lädt einen anderen Makrosatz', 'charge un autre jeu de macros', 'carga otro conjunto de macros', 'carrega outro conjunto de macros'),
  c('frequency', '<AUDIOFQ:Hz>', 'imposta la frequenza audio (con + o - è relativa)', 'sets the audio frequency (relative with + or -)', 'setzt die Audiofrequenz (mit + oder - relativ)', 'règle la fréquence audio (relative avec + ou -)', 'fija la frecuencia de audio (relativa con + o -)', 'define a frequência de áudio (relativa com + ou -)'),
  c('frequency', '<SEEKLEFT>', 'cerca un segnale verso sinistra', 'seeks a signal to the left', 'sucht ein Signal nach links', 'cherche un signal vers la gauche', 'busca una señal hacia la izquierda', 'procura um sinal para a esquerda'),
  c('frequency', '<SEEKRIGHT>', 'cerca un segnale verso destra', 'seeks a signal to the right', 'sucht ein Signal nach rechts', 'cherche un signal vers la droite', 'busca una señal hacia la derecha', 'procura um sinal para a direita'),
  c('frequency', '<BOOKMARK>', 'mette o toglie il segnalibro', 'toggles the bookmark', 'setzt oder entfernt das Lesezeichen', 'pose ou retire le signet', 'pone o quita el marcador', 'põe ou tira o marcador'),
  c('mode', '<MODE:PSK31>', 'cambia modo (PSK31, PSK63, RTTY)', 'changes mode (PSK31, PSK63, RTTY)', 'wechselt den Modus (PSK31, PSK63, RTTY)', 'change de mode (PSK31, PSK63, RTTY)', 'cambia de modo (PSK31, PSK63, RTTY)', 'muda de modo (PSK31, PSK63, RTTY)'),
  c('mode', '<BAUDRATE:n>', 'velocità: 31.25 o 62.5 per PSK, 45.45, 50, 75... per RTTY', 'speed: 31.25 or 62.5 for PSK, 45.45, 50, 75... for RTTY', 'Geschwindigkeit: 31,25 oder 62,5 für PSK, 45,45, 50, 75 ... für RTTY', 'vitesse : 31,25 ou 62,5 pour PSK, 45,45, 50, 75... pour RTTY', 'velocidad: 31,25 o 62,5 para PSK, 45,45, 50, 75... para RTTY', 'velocidade: 31,25 ou 62,5 para PSK, 45,45, 50, 75... para RTTY'),
  c('mode', '<SHIFT:Hz>', 'spostamento RTTY in Hz', 'RTTY shift in Hz', 'RTTY-Shift in Hz', 'shift RTTY en Hz', 'desplazamiento RTTY en Hz', 'desvio RTTY em Hz'),
];

/** Le voci di un gruppo (tutte se `group` e' vuoto). */
export function commandsOf(group: CommandGroup | ''): readonly MacroCommand[] {
  return group === '' ? MACRO_COMMANDS : MACRO_COMMANDS.filter((x) => x.group === group);
}
