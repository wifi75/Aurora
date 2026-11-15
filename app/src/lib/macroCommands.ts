/**
 * L'elenco dei comandi delle macro che Aurora conosce, unica fonte per la finestra di modifica e per la «Guida alle macro»: gruppo,
 * categoria, comando, esempio e spiegazione (breve e, dove serve, lunga) nelle sei lingue (it, en, de, fr, es, pt); l'olandese arriva
 * dal dizionario a parte per impronta del testo inglese (`pick`).
 */

/** I sottoinsiemi del menu «Subset»: testo, programma, frequenza, Auto CQ, macro, impostazioni del modo, rotore e i modi. */
export type CommandGroup = 'text' | 'program' | 'frequency' | 'autocq' | 'macros' | 'mode' | 'rotor' | 'rtty' | 'sstv' | 'cw' | 'hell';

export interface MacroCommand {
  group: CommandGroup;
  /** Il comando come si scrive nello script. */
  tag: string;
  /** La spiegazione: [it, en, de, fr, es, pt]. */
  desc: readonly [string, string, string, string, string, string];
  /** Il comando e' nell'elenco ma Aurora non lo esegue ancora. */
  soon?: boolean;
  /** Il comando non e' supportato per sicurezza: Aurora lo riconosce, lo dice e non fa niente (`<EXEC:...>`). */
  blocked?: boolean;
  /** La categoria nella Guida alle macro. */
  cat: HelpCategory;
  /** Un esempio di macro che lo usa. */
  ex: string;
  /** La spiegazione lunga, dove la breve non basta: [it, en, de, fr, es, pt]. */
  long?: readonly [string, string, string, string, string, string];
  /** Altri nomi (o forme) dello stesso comando. */
  aliases?: readonly string[];
}

type RawCommand = Pick<MacroCommand, 'group' | 'tag' | 'desc' | 'soon' | 'blocked'>;

const c = (group: CommandGroup, tag: string, ...desc: [string, string, string, string, string, string]): RawCommand => ({ group, tag, desc });
/** Un comando che Aurora non esegue ancora (si vede nell'elenco, con «non ancora»). */
const cs = (group: CommandGroup, tag: string, ...desc: [string, string, string, string, string, string]): RawCommand => ({ group, tag, desc, soon: true });

/** Un comando non supportato per sicurezza (si vede nell'elenco, con «non supportato per sicurezza»). */
const cb = (group: CommandGroup, tag: string, ...desc: [string, string, string, string, string, string]): RawCommand => ({ group, tag, desc, blocked: true });

const RAW_COMMANDS: readonly RawCommand[] = [
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
  c('text', '<NRS>', 'scambio da mandare (contest; vuoto = numero progressivo)', 'exchange to send (contest; empty = serial number)', 'zu sendender Austausch (Contest; leer = laufende Nummer)', 'échange à envoyer (concours ; vide = numéro de série)', 'intercambio a enviar (concurso; vacío = número de serie)', 'troca a enviar (concurso; vazio = número de série)'),
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
  c('text', '<QSONR>', 'numero progressivo del contest (001, avanza a ogni QSO salvato)', 'contest serial number (001, goes up with every saved QSO)', 'fortlaufende Contest-Nummer (001, steigt mit jedem gespeicherten QSO)', 'numéro de série du concours (001, augmente à chaque QSO enregistré)', 'número de serie del concurso (001, sube con cada QSO guardado)', 'número de série do concurso (001, sobe a cada QSO guardado)'),
  c('text', '<CR>', 'ritorno a capo (CR)', 'carriage return (CR)', 'Wagenrücklauf (CR)', 'retour chariot (CR)', 'retorno de carro (CR)', 'retorno de carro (CR)'),
  c('text', '<LF>', 'nuova riga (LF)', 'line feed (LF)', 'Zeilenvorschub (LF)', 'saut de ligne (LF)', 'salto de línea (LF)', 'avanço de linha (LF)'),
  c('text', '<CRLF>', 'a capo completo (CR + LF)', 'full new line (CR + LF)', 'voller Zeilenumbruch (CR + LF)', 'nouvelle ligne complète (CR + LF)', 'nueva línea completa (CR + LF)', 'nova linha completa (CR + LF)'),
  c('text', '<TAB>', 'tabulazione', 'tab character', 'Tabulator', 'tabulation', 'tabulación', 'tabulação'),
  c('text', '<ANYQSOBEFOREDATE>', 'data dell’ultimo QSO con questo nominativo nel log (vuoto se non c’è)', 'date of the last QSO with this callsign in the log (empty if none)', 'Datum des letzten QSO mit diesem Rufzeichen im Log (leer, wenn keins)', 'date du dernier QSO avec cet indicatif dans le log (vide s’il n’y en a pas)', 'fecha del último QSO con este indicativo en el log (vacío si no hay)', 'data do último QSO com este indicativo no log (vazio se não houver)'),
  c('text', '<ANYQSOBEFOREBAND>', 'banda dell’ultimo QSO con questo nominativo', 'band of the last QSO with this callsign', 'Band des letzten QSO mit diesem Rufzeichen', 'bande du dernier QSO avec cet indicatif', 'banda del último QSO con este indicativo', 'banda do último QSO com este indicativo'),
  c('text', '<ANYQSOBEFOREMODE>', 'modo dell’ultimo QSO con questo nominativo', 'mode of the last QSO with this callsign', 'Modus des letzten QSO mit diesem Rufzeichen', 'mode du dernier QSO avec cet indicatif', 'modo del último QSO con este indicativo', 'modo do último QSO com este indicativo'),
  c('text', '<PREVNRS>', 'contest: lo scambio che hai mandato nell’ultimo QSO del log (per ripeterlo a chi lo chiede di nuovo)', 'contest: the exchange you sent in the last logged QSO (to repeat it when asked again)', 'Contest: der Austausch, den du im letzten geloggten QSO gesendet hast (zum Wiederholen auf Nachfrage)', 'contest : l’échange envoyé dans le dernier QSO du log (pour le répéter sur demande)', 'contest: el intercambio que enviaste en el último QSO del log (para repetirlo si te lo piden)', 'contest: a troca que enviaste no último QSO do log (para a repetir quando pedida)'),
  c('text', '<QSOBEFORETIME>', 'contest: ora UTC del QSO già fatto con questo nominativo (vuoto se non c’è)', 'contest: UTC time of the earlier QSO with this callsign (empty if none)', 'Contest: UTC-Zeit des früheren QSO mit diesem Rufzeichen (leer, wenn keins)', 'contest : heure UTC du QSO déjà fait avec cet indicatif (vide sinon)', 'contest: hora UTC del QSO ya hecho con este indicativo (vacío si no hay)', 'contest: hora UTC do QSO já feito com este indicativo (vazio se não houver)'),
  c('text', '<QSOBEFOREDATE>', 'contest: data del QSO già fatto con questo nominativo', 'contest: date of the earlier QSO with this callsign', 'Contest: Datum des früheren QSO mit diesem Rufzeichen', 'contest : date du QSO déjà fait avec cet indicatif', 'contest: fecha del QSO ya hecho con este indicativo', 'contest: data do QSO já feito com este indicativo'),
  c('text', '<TEXTFILE:nome>', 'il testo di un file della cartella dei testi delle macro', 'the text of a file in the macro texts folder', 'der Text einer Datei im Ordner der Makrotexte', 'le texte d’un fichier du dossier des textes des macros', 'el texto de un archivo de la carpeta de textos de las macros', 'o texto de um ficheiro da pasta dos textos das macros'),
  c('text', '<[> <]>', 'i segni < e > nel testo', 'the < and > signs in the text', 'die Zeichen < und > im Text', 'les signes < et > dans le texte', 'los signos < y > en el texto', 'os sinais < e > no texto'),
  c('text', '<IMD>', 'IMD del segnale PSK ricevuto: Aurora la mostra nella barra ma il comando non scrive niente', 'IMD of the received PSK signal: Aurora shows it in the status bar but the command writes nothing', 'IMD des empfangenen PSK-Signals: Aurora zeigt sie in der Statusleiste, der Befehl schreibt aber nichts', 'IMD du signal PSK reçu : Aurora l’affiche dans la barre mais la commande n’écrit rien', 'IMD de la señal PSK recibida: Aurora la muestra en la barra pero el comando no escribe nada', 'IMD do sinal PSK recebido: o Aurora mostra-a na barra mas o comando não escreve nada'),
  c('text', '<VER>', 'nome del programma', 'program name', 'Programmname', 'nom du programme', 'nombre del programa', 'nome do programa'),
  cb('program', '<EXEC:file>', 'avvierebbe un programma esterno: Aurora non avvia programmi e toglie il comando dalle macro; la copia di sicurezza che facevano i vecchi .bat la fa Aurora da sola all’avvio', 'would start an external program: Aurora does not start programs and removes the command from the macros; the backup the old .bat files made is done by Aurora itself at start', 'würde ein externes Programm starten: Aurora startet keine Programme und entfernt den Befehl aus den Makros; die Sicherung der alten .bat-Dateien macht Aurora selbst beim Start', 'lancerait un programme externe : Aurora ne lance pas de programmes et retire la commande des macros ; la sauvegarde des anciens .bat, Aurora la fait seule au démarrage', 'iniciaría un programa externo: Aurora no inicia programas y quita el comando de las macros; la copia de seguridad de los antiguos .bat la hace Aurora sola al arrancar', 'iniciaria um programa externo: o Aurora não inicia programas e retira o comando das macros; a cópia de segurança dos antigos .bat o Aurora faz sozinho ao arrancar'),
  c('program', '<TX>', 'passa in trasmissione e manda il testo della macro (da solo: quello della casella)', 'switches to transmit and sends the macro text (alone: the text in the box)', 'schaltet auf Senden und sendet den Makrotext (allein: den Text im Feld)', 'passe en émission et envoie le texte de la macro (seul : celui de la case)', 'pasa a transmitir y envía el texto de la macro (solo: el de la casilla)', 'passa a transmitir e envia o texto da macro (sozinho: o da caixa)'),
  c('program', '<TXE>', 'trasmette quando cade il segnale sulla frequenza di ricezione (al massimo dopo 30 s; Esc annulla)', 'transmit when the signal on the receive frequency drops (after 30 s at most; Esc cancels)', 'sendet, wenn das Signal auf der Empfangsfrequenz wegfällt (spätestens nach 30 s; Esc bricht ab)', 'émet quand le signal sur la fréquence de réception tombe (au plus après 30 s ; Échap annule)', 'transmite cuando cae la señal en la frecuencia de recepción (como máximo tras 30 s; Esc cancela)', 'transmite quando cai o sinal na frequência de receção (no máximo após 30 s; Esc cancela)'),
  c('program', '<TXTOGGLE>', 'alterna fra trasmissione e ricezione (in trasmissione: torna in ricezione a fine testo)', 'toggles between transmit and receive (while sending: back to receive at the end of the text)', 'wechselt zwischen Senden und Empfang (beim Senden: zurück zum Empfang am Textende)', 'alterne entre émission et réception (en émission : retour en réception à la fin du texte)', 'alterna entre transmisión y recepción (transmitiendo: vuelve a recepción al final del texto)', 'alterna entre transmissão e receção (a transmitir: volta à receção no fim do texto)'),
  c('program', '<RX>', 'torna in ricezione quando la trasmissione arriva a questo punto; il testo dopo resta nella finestra senza partire', 'back to receive when the transmission reaches this point; the text after it stays in the window unsent', 'zurück zum Empfang, wenn die Sendung diese Stelle erreicht; der Text danach bleibt ungesendet im Fenster', 'retour en réception quand l’émission arrive à ce point ; le texte qui suit reste dans la fenêtre sans partir', 'vuelve a recepción cuando la transmisión llega a este punto; el texto que sigue queda en la ventana sin enviarse', 'volta à receção quando a transmissão chega a este ponto; o texto seguinte fica na janela sem ser enviado'),
  c('program', '<RXANDCLEAR>', 'come <RX> e, tornati in ricezione, pulisce la finestra di trasmissione', 'like <RX> and, once back to receive, clears the transmit window', 'wie <RX> und leert nach der Rückkehr zum Empfang das Sendefenster', 'comme <RX> et, de retour en réception, efface la fenêtre d’émission', 'como <RX> y, de vuelta en recepción, limpia la ventana de transmisión', 'como <RX> e, de volta à receção, limpa a janela de transmissão'),
  c('program', '<RXC>', 'uguale a <RXANDCLEAR>', 'same as <RXANDCLEAR>', 'wie <RXANDCLEAR>', 'identique à <RXANDCLEAR>', 'igual que <RXANDCLEAR>', 'igual a <RXANDCLEAR>'),
  c('program', '<RXE>', 'torna in ricezione a fine testo', 'back to receive when all text is sent', 'zurück zum Empfang nach dem ganzen Text', 'retour en réception à la fin du texte', 'vuelve a recepción al terminar el texto', 'volta à receção no fim do texto'),
  c('program', '<CLEARRXWINDOW>', 'pulisce la finestra di ricezione', 'clears the receive window', 'leert das Empfangsfenster', 'efface la fenêtre de réception', 'limpia la ventana de recepción', 'limpa a janela de receção'),
  c('program', '<CLEARTXWINDOW>', 'pulisce la finestra di trasmissione (casella e testo già partito) fino a questo punto: il testo dopo resta', 'clears the transmit window (box and text already sent) up to this point: the text after it stays', 'leert das Sendefenster (Feld und bereits gesendeten Text) bis zu dieser Stelle: der Text danach bleibt', 'efface la fenêtre d’émission (case et texte déjà émis) jusqu’à ce point : le texte qui suit reste', 'limpia la ventana de transmisión (casilla y texto ya enviado) hasta este punto: el texto que sigue queda', 'limpa a janela de transmissão (caixa e texto já enviado) até este ponto: o texto seguinte fica'),
  c('program', '<CLEARWINDOW>', 'pulisce le finestre di ricezione e di trasmissione', 'clears the receive and transmit windows', 'leert Empfangs- und Sendefenster', 'efface les fenêtres de réception et d’émission', 'limpia las ventanas de recepción y de transmisión', 'limpa as janelas de receção e de transmissão'),
  c('program', '<CLEARQSO>', 'azzera i dati del QSO', 'clears the QSO data', 'löscht die QSO-Daten', 'efface les données du QSO', 'borra los datos del QSO', 'apaga os dados do QSO'),
  c('program', '<GRABCALL>', 'prende l’ultimo nominativo ricevuto nella finestra RX (non il tuo) e lo mette nel QSO: il <CALL> che segue lo usa subito', 'takes the last callsign received in the RX window (not yours) and puts it in the QSO: the <CALL> after it uses it at once', 'übernimmt das zuletzt im RX-Fenster empfangene Rufzeichen (nicht deins) ins QSO: das folgende <CALL> nutzt es sofort', 'prend le dernier indicatif reçu dans la fenêtre RX (pas le tien) et le met dans le QSO : le <CALL> qui suit l’utilise aussitôt', 'toma el último indicativo recibido en la ventana RX (no el tuyo) y lo pone en el QSO: el <CALL> que sigue lo usa enseguida', 'pega o último indicativo recebido na janela RX (não o teu) e põe-no no QSO: o <CALL> seguinte usa-o logo'),
  c('program', '<FIXSTARTTIME>', 'fissa adesso come ora di inizio del QSO', 'fixes now as the QSO start time', 'setzt jetzt als Beginn des QSO', 'fixe maintenant comme début du QSO', 'fija ahora como inicio del QSO', 'fixa agora como início do QSO'),
  c('program', '<SAVEQSO>', 'salva il QSO a log (va prima dei comandi di pulizia)', 'saves the QSO to the log (put it before any clear command)', 'speichert das QSO im Log (vor den Löschbefehlen)', 'enregistre le QSO au journal (avant les commandes d’effacement)', 'guarda el QSO en el log (antes de los comandos de limpieza)', 'guarda o QSO no log (antes dos comandos de limpeza)'),
  c('program', '<QTCSEND>', 'apre la finestra per mandare i QTC (contest WAE) alla stazione del QSO', 'opens the window to send QTCs (WAE contest) to the station of the QSO', 'öffnet das Fenster zum Senden der QTCs (WAE-Contest) an die Station des QSO', 'ouvre la fenêtre pour envoyer les QTC (concours WAE) à la station du QSO', 'abre la ventana para enviar los QTC (concurso WAE) a la estación del QSO', 'abre a janela para enviar os QTC (concurso WAE) à estação do QSO'),
  c('program', '<QTCREAD>', 'apre la finestra per ricevere una serie di QTC (contest WAE) dalla stazione del QSO', 'opens the window to receive a QTC series (WAE contest) from the station of the QSO', 'öffnet das Fenster zum Empfang einer QTC-Serie (WAE-Contest) von der Station des QSO', 'ouvre la fenêtre pour recevoir une série de QTC (concours WAE) de la station du QSO', 'abre la ventana para recibir una serie de QTC (concurso WAE) de la estación del QSO', 'abre a janela para receber uma série de QTC (concurso WAE) da estação do QSO'),
  c('autocq', '<AUTOCQ>', 'avvia o ferma la chiamata CQ automatica; seguito da un testo, quel testo diventa la chiamata', 'starts or stops the automatic CQ; followed by a text, that text becomes the call', 'startet oder stoppt den automatischen CQ-Ruf; folgt ein Text, wird er zum Ruf', 'démarre ou arrête l’appel CQ automatique ; suivi d’un texte, ce texte devient l’appel', 'inicia o detiene la llamada CQ automática; seguido de un texto, ese texto pasa a ser la llamada', 'inicia ou pára a chamada CQ automática; seguido de um texto, esse texto passa a ser a chamada'),
  cs('program', '<TUNE>', 'tono per accordare: nei modi testuali Aurora non lo manda (usa il tasto Tune della radio)', 'tuning tone: Aurora does not send it in the text modes (use the radio’s Tune key)', 'Abstimmton: in den Textmodi sendet Aurora ihn nicht (Tune-Taste des Funkgeräts verwenden)', 'porteuse d’accord : Aurora ne l’envoie pas dans les modes texte (utilise la touche Tune de la radio)', 'tono de sintonía: Aurora no lo envía en los modos de texto (usa la tecla Tune de la radio)', 'tom de sintonia: o Aurora não o envia nos modos de texto (usa a tecla Tune do rádio)'),
  c('program', '<BEEP:ON>', 'suona un avviso', 'sounds a beep', 'gibt einen Signalton aus', 'émet un bip', 'emite un pitido', 'emite um aviso sonoro'),
  c('program', '<CURSOR:TX>', 'porta il cursore nella casella di trasmissione', 'puts the cursor in the transmit box', 'setzt den Cursor ins Sendefeld', 'place le curseur dans la case d’émission', 'pone el cursor en la casilla de transmisión', 'põe o cursor na caixa de transmissão'),
  cs('macros', '<MACROS:file>', 'carica un altro insieme di macro: Aurora non ha file di macro (le macro di ogni modo stanno nelle Impostazioni)', 'loads another macro set: Aurora has no macro files (each mode’s macros are in Settings)', 'lädt einen anderen Makrosatz: Aurora hat keine Makrodateien (die Makros jedes Modus stehen in den Einstellungen)', 'charge un autre jeu de macros : Aurora n’a pas de fichiers de macros (les macros de chaque mode sont dans les Réglages)', 'carga otro conjunto de macros: Aurora no tiene archivos de macros (las de cada modo están en Ajustes)', 'carrega outro conjunto de macros: o Aurora não tem ficheiros de macros (as de cada modo estão nas Definições)'),
  c('frequency', '<AUDIOFQ:Hz>', 'imposta la frequenza audio (con + o - è relativa)', 'sets the audio frequency (relative with + or -)', 'setzt die Audiofrequenz (mit + oder - relativ)', 'règle la fréquence audio (relative avec + ou -)', 'fija la frecuencia de audio (relativa con + o -)', 'define a frequência de áudio (relativa com + ou -)'),
  c('frequency', '<SEEKLEFT>', 'cerca un segnale verso sinistra', 'seeks a signal to the left', 'sucht ein Signal nach links', 'cherche un signal vers la gauche', 'busca una señal hacia la izquierda', 'procura um sinal para a esquerda'),
  c('frequency', '<SEEKRIGHT>', 'cerca un segnale verso destra', 'seeks a signal to the right', 'sucht ein Signal nach rechts', 'cherche un signal vers la droite', 'busca una señal hacia la derecha', 'procura um sinal para a direita'),
  cs('frequency', '<BOOKMARK>', 'segnalibro sulla cascata: Aurora non ne ha', 'bookmark on the waterfall: Aurora has none', 'Lesezeichen im Wasserfall: Aurora hat keine', 'signet sur la cascade : Aurora n’en a pas', 'marcador en la cascada: Aurora no tiene', 'marcador na cascata: o Aurora não tem'),
  c('mode', '<MODE:PSK31>', 'cambia modo (PSK31, PSK63, RTTY)', 'changes mode (PSK31, PSK63, RTTY)', 'wechselt den Modus (PSK31, PSK63, RTTY)', 'change de mode (PSK31, PSK63, RTTY)', 'cambia de modo (PSK31, PSK63, RTTY)', 'muda de modo (PSK31, PSK63, RTTY)'),
  c('rtty', '<BAUDRATE:n>', 'velocità: 31.25 o 62.5 per PSK, 45.45, 50, 75... per RTTY', 'speed: 31.25 or 62.5 for PSK, 45.45, 50, 75... for RTTY', 'Geschwindigkeit: 31,25 oder 62,5 für PSK, 45,45, 50, 75 ... für RTTY', 'vitesse : 31,25 ou 62,5 pour PSK, 45,45, 50, 75... pour RTTY', 'velocidad: 31,25 o 62,5 para PSK, 45,45, 50, 75... para RTTY', 'velocidade: 31,25 ou 62,5 para PSK, 45,45, 50, 75... para RTTY'),
  c('rtty', '<SHIFT:Hz>', 'spostamento RTTY in Hz', 'RTTY shift in Hz', 'RTTY-Shift in Hz', 'shift RTTY en Hz', 'desplazamiento RTTY en Hz', 'desvio RTTY em Hz'),
  // Auto CQ
  c('autocq', '<ACQ>', 'come <AUTOCQ>', 'same as <AUTOCQ>', 'wie <AUTOCQ>', 'comme <AUTOCQ>', 'igual que <AUTOCQ>', 'igual a <AUTOCQ>'),
  c('autocq', '<ALARM:testo>', 'mostra un avviso con il testo', 'shows a notice with the text', 'zeigt einen Hinweis mit dem Text', 'affiche un avis avec le texte', 'muestra un aviso con el texto', 'mostra um aviso com o texto'),
  c('autocq', '<ACQDELAY:s>', 'pausa fra due CQ automatici (secondi)', 'pause between two automatic CQs (seconds)', 'Pause zwischen zwei automatischen CQ (Sekunden)', 'pause entre deux CQ automatiques (secondes)', 'pausa entre dos CQ automáticos (segundos)', 'pausa entre dois CQ automáticos (segundos)'),
  c('autocq', '<ASAUTOCQ>', 'accende l’Auto CQ; il testo che segue diventa la chiamata', 'turns Auto CQ on; the text that follows becomes the call', 'schaltet Auto-CQ ein; der folgende Text wird zum Ruf', 'active l’Auto CQ ; le texte qui suit devient l’appel', 'activa el Auto CQ; el texto que sigue pasa a ser la llamada', 'liga o Auto CQ; o texto seguinte passa a ser a chamada'),
  c('autocq', '<AUTOCQRESUME>', 'riprende l’Auto CQ fermato', 'resumes a stopped Auto CQ', 'setzt einen gestoppten Auto-CQ fort', 'reprend un Auto CQ arrêté', 'reanuda un Auto CQ detenido', 'retoma um Auto CQ parado'),
  // Macro
  c('macros', '<EDITMACRO:nome>', 'apre la finestra di modifica di una macro (F5, Ctrl-F5...)', 'opens the edit window of a macro (F5, Ctrl-F5...)', 'öffnet das Bearbeitungsfenster eines Makros (F5, Ctrl-F5 ...)', 'ouvre la fenêtre de modification d’une macro (F5, Ctrl-F5...)', 'abre la ventana de edición de una macro (F5, Ctrl-F5...)', 'abre a janela de edição de uma macro (F5, Ctrl-F5...)'),
  cs('macros', '<MODEMACROS:file>', 'carica le macro di un altro modo', 'loads the macros of another mode', 'lädt die Makros eines anderen Modus', 'charge les macros d’un autre mode', 'carga las macros de otro modo', 'carrega as macros de outro modo'),
  cs('macros', '<HIDETEXT>', 'nasconde il testo che si trasmette', 'hides the text being sent', 'blendet den gesendeten Text aus', 'masque le texte émis', 'oculta el texto que se transmite', 'oculta o texto que se transmite'),
  cs('macros', '<SHOWTEXT>', 'mostra di nuovo il testo che si trasmette', 'shows the text being sent again', 'zeigt den gesendeten Text wieder', 'réaffiche le texte émis', 'vuelve a mostrar el texto que se transmite', 'volta a mostrar o texto que se transmite'),
  // Frequenza
  c('frequency', '<FQ:kHz>', 'porta la radio in modo che il segnale cada su questa frequenza in kHz', 'tunes the radio so the signal falls on this frequency in kHz', 'stellt das Funkgerät so ein, dass das Signal auf dieser Frequenz in kHz liegt', 'règle la radio pour que le signal tombe sur cette fréquence en kHz', 'sintoniza la radio para que la señal caiga en esta frecuencia en kHz', 'sintoniza o rádio para que o sinal caia nesta frequência em kHz'),
  c('frequency', '<SPLIT:1>', 'split acceso (1) o spento (0): la trasmissione resta dov’è mentre sposti l’ascolto', 'split on (1) or off (0): the transmit stays put while you move the receive', 'Split an (1) oder aus (0): das Senden bleibt, während du den Empfang verschiebst', 'split activé (1) ou coupé (0) : l’émission reste en place pendant que tu déplaces la réception', 'split activado (1) o apagado (0): la transmisión se queda mientras mueves la recepción', 'split ligado (1) ou desligado (0): a transmissão fica onde está enquanto moves a receção'),
  c('frequency', '<JUMPTOTXFQ>', 'porta l’ascolto sulla frequenza di trasmissione', 'moves the receive to the transmit frequency', 'setzt den Empfang auf die Sendefrequenz', 'place la réception sur la fréquence d’émission', 'lleva la recepción a la frecuencia de transmisión', 'leva a receção para a frequência de transmissão'),
  c('frequency', '<ALIGN:Hz>', 'allinea la frequenza di ricezione a un multiplo di questi Hz', 'aligns the receive frequency to a multiple of these Hz', 'richtet die Empfangsfrequenz an einem Vielfachen dieser Hz aus', 'aligne la fréquence de réception sur un multiple de ces Hz', 'alinea la frecuencia de recepción a un múltiplo de estos Hz', 'alinha a frequência de receção a um múltiplo destes Hz'),
  c('frequency', '<CATCMD:comando>', 'manda un comando CAT alla radio', 'sends a CAT command to the radio', 'sendet einen CAT-Befehl an das Funkgerät', 'envoie une commande CAT à la radio', 'envía un comando CAT a la radio', 'envia um comando CAT ao rádio'),
  // Impostazioni del modo
  c('mode', '<AFCON>', 'accende l’AFC (<AFCOFF> lo spegne, <AFC> lo alterna)', 'turns AFC on (<AFCOFF> off, <AFC> toggles)', 'schaltet AFC ein (<AFCOFF> aus, <AFC> wechselt)', 'active l’AFC (<AFCOFF> le coupe, <AFC> bascule)', 'activa el AFC (<AFCOFF> lo apaga, <AFC> alterna)', 'liga o AFC (<AFCOFF> desliga, <AFC> alterna)'),
  c('mode', '<SQUELCHON>', 'accende lo squelch (<SQUELCHOFF>, <SQUELCH>)', 'turns squelch on (<SQUELCHOFF>, <SQUELCH>)', 'schaltet Squelch ein (<SQUELCHOFF>, <SQUELCH>)', 'active le squelch (<SQUELCHOFF>, <SQUELCH>)', 'activa el squelch (<SQUELCHOFF>, <SQUELCH>)', 'liga o squelch (<SQUELCHOFF>, <SQUELCH>)'),
  c('mode', '<SNAPON>', 'accende Snap (<SNAPOFF>, <SNAP>)', 'turns Snap on (<SNAPOFF>, <SNAP>)', 'schaltet Snap ein (<SNAPOFF>, <SNAP>)', 'active Snap (<SNAPOFF>, <SNAP>)', 'activa Snap (<SNAPOFF>, <SNAP>)', 'liga o Snap (<SNAPOFF>, <SNAP>)'),
  c('mode', '<LOCKTX>', 'blocca la trasmissione dov’è (<UNLOCKTX> la libera)', 'locks the transmit where it is (<UNLOCKTX> frees it)', 'hält das Senden fest (<UNLOCKTX> gibt es frei)', 'fixe l’émission où elle est (<UNLOCKTX> la libère)', 'fija la transmisión donde está (<UNLOCKTX> la libera)', 'fixa a transmissão onde está (<UNLOCKTX> liberta-a)'),
  c('program', '<IGNOREBEFORE:aaaa-mm-gg hh:mm>', 'i segni del log (BIGScan, fuori dal contest) non contano i QSO prima di questa data e ora UTC; <IGNOREBEFORE:ERASE> toglie il limite', 'log marks (BIGScan, outside a contest) ignore QSOs before this UTC date and time; <IGNOREBEFORE:ERASE> removes the limit', 'Log-Markierungen (BIGScan, außerhalb eines Contests) ignorieren QSOs vor diesem UTC-Datum und dieser Uhrzeit; <IGNOREBEFORE:ERASE> hebt die Grenze auf', 'les repères du log (BIGScan, hors concours) ignorent les QSO avant cette date et heure UTC ; <IGNOREBEFORE:ERASE> retire la limite', 'las marcas del log (BIGScan, fuera de un concurso) ignoran los QSO anteriores a esta fecha y hora UTC; <IGNOREBEFORE:ERASE> quita el límite', 'as marcas do log (BIGScan, fora de um concurso) ignoram os QSO antes desta data e hora UTC; <IGNOREBEFORE:ERASE> tira o limite'),
  c('program', '<PERIOD:n>', 'i segni del log (BIGScan, fuori dal contest) contano solo i QSO degli ultimi n minuti; 0 = spento', 'log marks (BIGScan, outside a contest) count only QSOs from the last n minutes; 0 = off', 'Log-Markierungen (BIGScan, außerhalb eines Contests) zählen nur QSOs der letzten n Minuten; 0 = aus', 'les repères du log (BIGScan, hors concours) ne comptent que les QSO des n dernières minutes ; 0 = désactivé', 'las marcas del log (BIGScan, fuera de un concurso) solo cuentan los QSO de los últimos n minutos; 0 = apagado', 'as marcas do log (BIGScan, fora de um concurso) só contam os QSO dos últimos n minutos; 0 = desligado'),
  c('frequency', '<STARTSCAN>', 'avvia la scansione della banda (<STOPSCAN> la ferma)', 'starts the band scan (<STOPSCAN> stops it)', 'startet den Bandscan (<STOPSCAN> stoppt ihn)', 'lance le balayage de la bande (<STOPSCAN> l’arrête)', 'inicia el barrido de la banda (<STOPSCAN> lo detiene)', 'inicia a varredura da banda (<STOPSCAN> pára-a)'),
  c('mode', '<MODESETTINGS>', 'apre le impostazioni del modo', 'opens the mode settings', 'öffnet die Moduseinstellungen', 'ouvre les réglages du mode', 'abre los ajustes del modo', 'abre as definições do modo'),
  c('mode', '<THRESHOLD>', 'apre la soglia dello squelch', 'opens the squelch threshold', 'öffnet die Squelch-Schwelle', 'ouvre le seuil du squelch', 'abre el umbral del squelch', 'abre o limiar do squelch'),
  c('mode', '<MEMW:n>', 'salva le impostazioni nella memoria n (<MEMR:n> la rilegge)', 'saves the settings in memory n (<MEMR:n> reads it back)', 'speichert die Einstellungen im Speicher n (<MEMR:n> liest sie zurück)', 'enregistre les réglages dans la mémoire n (<MEMR:n> la relit)', 'guarda los ajustes en la memoria n (<MEMR:n> la vuelve a leer)', 'guarda as definições na memória n (<MEMR:n> volta a lê-la)'),
  // Rotore
  c('rotor', '<ROTOR:gradi>', 'gira il rotore su quella direzione (0-359)', 'turns the rotator to that heading (0-359)', 'dreht den Rotor auf diese Richtung (0-359)', 'tourne le rotor vers ce cap (0-359)', 'gira el rotor hacia ese rumbo (0-359)', 'roda o rotor para esse rumo (0-359)'),
  c('rotor', '<ROTOR:SP>', 'gira il rotore verso la stazione del QSO per la via corta', 'turns the rotator toward the QSO station by the short path', 'dreht den Rotor zur QSO-Station auf dem kurzen Weg', 'tourne le rotor vers la station du QSO par le chemin court', 'gira el rotor hacia la estación del QSO por el camino corto', 'roda o rotor para a estação do QSO pelo caminho curto'),
  c('rotor', '<ROTOR:LP>', 'come <ROTOR:SP> ma per la via lunga', 'like <ROTOR:SP> but by the long path', 'wie <ROTOR:SP>, aber auf dem langen Weg', 'comme <ROTOR:SP> mais par le grand chemin', 'como <ROTOR:SP> pero por el camino largo', 'como <ROTOR:SP> mas pelo caminho longo'),
  // RTTY
  cs('rtty', '<LETTERS>', 'manda il codice di passaggio alle lettere', 'sends the shift-to-letters code', 'sendet den Umschaltcode auf Buchstaben', 'envoie le code de passage aux lettres', 'envía el código de paso a letras', 'envia o código de passagem a letras'),
  cs('rtty', '<NUMBERS>', 'manda il codice di passaggio alle cifre', 'sends the shift-to-figures code', 'sendet den Umschaltcode auf Ziffern', 'envoie le code de passage aux chiffres', 'envía el código de paso a cifras', 'envia o código de passagem a algarismos'),
  // SSTV
  cs('sstv', '<LOADPICTURE>', 'carica un’immagine da trasmettere', 'loads a picture to send', 'lädt ein Bild zum Senden', 'charge une image à émettre', 'carga una imagen para transmitir', 'carrega uma imagem para transmitir'),
  cs('sstv', '<FROMCLIP>', 'prende l’immagine dagli appunti', 'takes the picture from the clipboard', 'nimmt das Bild aus der Zwischenablage', 'prend l’image du presse-papiers', 'toma la imagen del portapapeles', 'toma a imagem da área de transferência'),
  cs('sstv', '<FONT:nome>', 'sceglie il carattere per i testi sull’immagine', 'chooses the font for the texts on the picture', 'wählt die Schrift für die Texte im Bild', 'choisit la police des textes sur l’image', 'elige la fuente para los textos de la imagen', 'escolhe o tipo de letra para os textos da imagem'),
  cs('sstv', '<LIMITSCANLINES:n>', 'limita le righe trasmesse', 'limits the scan lines sent', 'begrenzt die gesendeten Zeilen', 'limite les lignes émises', 'limita las líneas transmitidas', 'limita as linhas transmitidas'),
  // CW
  c('cw', '<AR> <BT> <KN> <SK>', 'segni di procedura: in CW un segno solo, negli altri modi le lettere', 'procedure signs: one sign in CW, the letters in other modes', 'Betriebszeichen: in CW ein Zeichen, in anderen Modi die Buchstaben', 'signes de procédure : un seul signe en CW, les lettres dans les autres modes', 'signos de procedimiento: un solo signo en CW, las letras en los otros modos', 'sinais de procedimento: um só sinal em CW, as letras nos outros modos'),
  c('cw', '<WPM:n>', 'velocità in parole al minuto (con + o - è relativa)', 'speed in words per minute (relative with + or -)', 'Geschwindigkeit in Wörtern pro Minute (mit + oder - relativ)', 'vitesse en mots par minute (relative avec + ou -)', 'velocidad en palabras por minuto (relativa con + o -)', 'velocidade em palavras por minuto (relativa com + ou -)'),
  cs('cw', '<LANGUAGE:xx>', 'tabella dei caratteri della lingua', 'character table of the language', 'Zeichentabelle der Sprache', 'table de caractères de la langue', 'tabla de caracteres del idioma', 'tabela de caracteres do idioma'),
  cs('cw', '<WK:comando>', 'manda un comando al Winkeyer', 'sends a command to the Winkeyer', 'sendet einen Befehl an den Winkeyer', 'envoie une commande au Winkeyer', 'envía un comando al Winkeyer', 'envia um comando ao Winkeyer'),
  // HELL
  cs('hell', '<SET FILTER:n>', 'filtro del ricevitore Hell', 'Hell receiver filter', 'Filter des Hell-Empfängers', 'filtre du récepteur Hell', 'filtro del receptor Hell', 'filtro do recetor Hell'),
  cs('hell', '<SET FONT:nome>', 'carattere della trasmissione Hell', 'font of the Hell transmission', 'Schrift der Hell-Aussendung', 'police de l’émission Hell', 'fuente de la transmisión Hell', 'tipo de letra da transmissão Hell'),
  cs('hell', '<SET SUBMODE:n>', 'sottomodo di Hellschreiber', 'Hellschreiber submode', 'Untermodus von Hellschreiber', 'sous-mode de Hellschreiber', 'submodo de Hellschreiber', 'submodo de Hellschreiber'),
];

/** Le categorie della «Guida alle macro» (pagina dei modi testuali). */
export type HelpCategory = 'data' | 'time' | 'format' | 'freq' | 'txrx' | 'windows' | 'qso' | 'mode' | 'autocq' | 'rotor' | 'cat' | 'files' | 'cw' | 'rtty' | 'sstv' | 'hell';

export const HELP_CATEGORIES: readonly HelpCategory[] = ['data', 'time', 'format', 'freq', 'txrx', 'windows', 'qso', 'mode', 'autocq', 'rotor', 'cat', 'files', 'cw', 'rtty', 'sstv', 'hell'];

const CATEGORY: Record<string, HelpCategory> = {
  '<MYCALL>': 'data', '<MYMAINCALL>': 'data', '<MYNAME>': 'data', '<MYQTH>': 'data', '<MYWWLOC>': 'data', '<CALL>': 'data', '<NAME>': 'data',
  '<QTH>': 'data', '<WWLOC>': 'data', '<RSTS>': 'data', '<RSTR>': 'data', '<NOTES>': 'data', '<ANYQSOBEFOREDATE>': 'data',
  '<ANYQSOBEFOREBAND>': 'data', '<ANYQSOBEFOREMODE>': 'data', '<PREVNRS>': 'data', '<QSOBEFORETIME>': 'data', '<QSOBEFOREDATE>': 'data', '<IMD>': 'data', '<VER>': 'data',
  '<DATE>': 'time', '<TIME>': 'time', '<LOCALTIME>': 'time', '<STARTTIME>': 'time', '<CTIME>': 'time', '<GA>': 'time', '<GAL>': 'time',
  '<CR>': 'format', '<LF>': 'format', '<CRLF>': 'format', '<TAB>': 'format', '<[> <]>': 'format',
  '<KHZ>': 'freq', '<MHZ>': 'freq', '<BAND>': 'freq', '<AUDIOFQ:Hz>': 'freq', '<SEEKLEFT>': 'freq', '<SEEKRIGHT>': 'freq', '<BOOKMARK>': 'freq',
  '<FQ:kHz>': 'freq', '<SPLIT:1>': 'freq', '<JUMPTOTXFQ>': 'freq', '<ALIGN:Hz>': 'freq', '<STARTSCAN>': 'freq',
  '<TX>': 'txrx', '<TXE>': 'txrx', '<TXTOGGLE>': 'txrx', '<RX>': 'txrx', '<RXANDCLEAR>': 'txrx', '<RXC>': 'txrx', '<RXE>': 'txrx', '<TUNE>': 'txrx',
  '<CLEARRXWINDOW>': 'windows', '<CLEARTXWINDOW>': 'windows', '<CLEARWINDOW>': 'windows', '<CURSOR:TX>': 'windows', '<HIDETEXT>': 'windows',
  '<SHOWTEXT>': 'windows', '<BEEP:ON>': 'windows', '<ALARM:testo>': 'windows',
  '<NRS>': 'qso', '<NRR>': 'qso', '<QSONR>': 'qso', '<CLEARQSO>': 'qso', '<GRABCALL>': 'qso', '<FIXSTARTTIME>': 'qso', '<SAVEQSO>': 'qso', '<QTCSEND>': 'qso', '<QTCREAD>': 'qso',
  '<IGNOREBEFORE:aaaa-mm-gg hh:mm>': 'qso', '<PERIOD:n>': 'qso',
  '<MODE>': 'mode', '<MODE:PSK31>': 'mode', '<BAUDRATE:n>': 'mode', '<SHIFT:Hz>': 'mode', '<AFCON>': 'mode', '<SQUELCHON>': 'mode', '<SNAPON>': 'mode',
  '<LOCKTX>': 'mode', '<MODESETTINGS>': 'mode', '<THRESHOLD>': 'mode', '<MEMW:n>': 'mode',
  '<AUTOCQ>': 'autocq', '<ACQ>': 'autocq', '<ACQDELAY:s>': 'autocq', '<ASAUTOCQ>': 'autocq', '<AUTOCQRESUME>': 'autocq',
  '<ROTOR:gradi>': 'rotor', '<ROTOR:SP>': 'rotor', '<ROTOR:LP>': 'rotor',
  '<CATCMD:comando>': 'cat',
  '<TEXTFILE:nome>': 'files', '<MACROS:file>': 'files', '<EXEC:file>': 'files', '<EDITMACRO:nome>': 'files', '<MODEMACROS:file>': 'files',
  '<AR> <BT> <KN> <SK>': 'cw', '<WPM:n>': 'cw', '<LANGUAGE:xx>': 'cw', '<WK:comando>': 'cw',
  '<LETTERS>': 'rtty', '<NUMBERS>': 'rtty',
  '<LOADPICTURE>': 'sstv', '<FROMCLIP>': 'sstv', '<FONT:nome>': 'sstv', '<LIMITSCANLINES:n>': 'sstv',
  '<SET FILTER:n>': 'hell', '<SET FONT:nome>': 'hell', '<SET SUBMODE:n>': 'hell',
};

/** Un esempio di uso per ogni comando (testo di macro: non si traduce). */
const EXAMPLE: Record<string, string> = {
  '<MYCALL>': 'CQ CQ DE <MYCALL> <MYCALL> K', '<MYMAINCALL>': 'DE <MYMAINCALL>', '<MYNAME>': 'NAME <MYNAME>', '<MYQTH>': 'QTH <MYQTH>',
  '<MYWWLOC>': 'LOC <MYWWLOC>', '<CALL>': '<CALL> DE <MYCALL> KN', '<NAME>': 'TNX <NAME> FER QSO', '<QTH>': 'UR QTH <QTH>', '<WWLOC>': 'UR LOC <WWLOC>',
  '<RSTS>': 'UR RST <RSTS> <RSTS>', '<RSTR>': 'TNX RPT <RSTR>', '<NOTES>': '<NOTES>', '<ANYQSOBEFOREDATE>': 'LAST QSO <ANYQSOBEFOREDATE> ON <ANYQSOBEFOREBAND>',
  '<ANYQSOBEFOREBAND>': 'LAST QSO ON <ANYQSOBEFOREBAND>', '<ANYQSOBEFOREMODE>': 'LAST QSO IN <ANYQSOBEFOREMODE>', '<PREVNRS>': 'NR <PREVNRS> <PREVNRS>', '<QSOBEFORETIME>': 'QSO B4 <QSOBEFORETIME>', '<QSOBEFOREDATE>': 'QSO B4 <QSOBEFOREDATE>', '<IMD>': 'IMD <IMD>', '<VER>': 'SW <VER>',
  '<DATE>': 'DATE <DATE>', '<TIME>': 'TIME <TIME> UTC', '<LOCALTIME>': 'LOCAL TIME <LOCALTIME>', '<STARTTIME>': 'QSO STARTED <STARTTIME>', '<CTIME>': '<CTIME>Z',
  '<GA>': '<GA> <NAME>', '<GAL>': '<GAL> <NAME>',
  '<CR>': 'line 1<CR>', '<LF>': 'line 1<LF>line 2', '<CRLF>': '<CALL> DE <MYCALL><CRLF>', '<TAB>': 'NAME<TAB><NAME>', '<[> <]>': '<[>73<]>',
  '<KHZ>': 'QRV <KHZ:1> KHZ', '<MHZ>': 'QRG <MHZ:3> MHZ', '<BAND>': 'ON <BAND>', '<AUDIOFQ:Hz>': '<AUDIOFQ:1500>', '<SEEKLEFT>': '<SEEKLEFT>',
  '<SEEKRIGHT>': '<SEEKRIGHT>', '<BOOKMARK>': '<BOOKMARK>', '<FQ:kHz>': '<FQ:14070.5>', '<SPLIT:1>': '<SPLIT:1>', '<JUMPTOTXFQ>': '<JUMPTOTXFQ>',
  '<ALIGN:Hz>': '<ALIGN:50>', '<STARTSCAN>': '<STARTSCAN>',
  '<TX>': '<TX>CQ CQ DE <MYCALL> K<RX>', '<TXE>': '<TXE><CALL> DE <MYCALL> KN<RX>', '<TXTOGGLE>': '<TXTOGGLE>', '<RX>': '<TX>73 <CALL> DE <MYCALL> SK<RX>',
  '<RXANDCLEAR>': '<TX>TU 73 DE <MYCALL> SK<RXANDCLEAR>', '<RXC>': '<TX>TU <MYCALL><RXC>', '<RXE>': '<TX><RXE>QRZ? DE <MYCALL> K', '<TUNE>': '<TUNE:5>',
  '<CLEARRXWINDOW>': '<CLEARRXWINDOW>', '<CLEARTXWINDOW>': '<CLEARTXWINDOW><TX>CQ DE <MYCALL> K<RX>', '<CLEARWINDOW>': '<CLEARWINDOW>',
  '<CURSOR:TX>': '<CALL> DE <MYCALL> <CURSOR:TX>', '<HIDETEXT>': '<HIDETEXT>', '<SHOWTEXT>': '<SHOWTEXT>', '<BEEP:ON>': '<BEEP:ON>',
  '<ALARM:testo>': '<ALARM:QSO saved>',
  '<NRS>': '<TX><CALL> 599 <NRS> <NRS><RX>', '<NRR>': 'TNX <NRR>', '<QSONR>': '599 <QSONR>', '<CLEARQSO>': '<SAVEQSO><CLEARQSO>', '<GRABCALL>': '<GRABCALL><TX><CALL> TU 599 <NRS><RX>', '<FIXSTARTTIME>': '<FIXSTARTTIME><TX><CALL> DE <MYCALL><RX>',
  '<SAVEQSO>': '<TX>TU <MYCALL><RX><SAVEQSO><CLEARQSO>', '<QTCSEND>': '<QTCSEND>', '<QTCREAD>': '<QTCREAD>', '<IGNOREBEFORE:aaaa-mm-gg hh:mm>': '<IGNOREBEFORE:2026-11-14 12:00>', '<PERIOD:n>': '<PERIOD:60>',
  '<MODE>': 'MODE <MODE>', '<MODE:PSK31>': '<MODE:RTTY>', '<BAUDRATE:n>': '<BAUDRATE:45.45>', '<SHIFT:Hz>': '<SHIFT:170>', '<AFCON>': '<AFCOFF>', '<SQUELCHON>': '<SQUELCH>',
  '<SNAPON>': '<SNAPON>', '<LOCKTX>': '<LOCKTX>', '<MODESETTINGS>': '<MODESETTINGS>', '<THRESHOLD>': '<THRESHOLD>', '<MEMW:n>': '<MEMW:1>',
  '<AUTOCQ>': '<AUTOCQ>CQ CQ DE <MYCALL> <MYCALL> K<RX>', '<ACQ>': '<ACQ>', '<ACQDELAY:s>': '<ACQDELAY:15>', '<ASAUTOCQ>': '<ASAUTOCQ>CQ TEST <MYCALL><RX>',
  '<AUTOCQRESUME>': '<AUTOCQRESUME>',
  '<ROTOR:gradi>': '<ROTOR:45>', '<ROTOR:SP>': '<ROTOR:SP>', '<ROTOR:LP>': '<ROTOR:LP>',
  '<CATCMD:comando>': '<CATCMD:AN1;>',
  '<TEXTFILE:nome>': '<TX><TEXTFILE:station.txt><RX>', '<MACROS:file>': '<MACROS:contest.mc>', '<EXEC:file>': '<EXEC:Backup.bat>', '<EDITMACRO:nome>': '<EDITMACRO:F5>', '<MODEMACROS:file>': '<MODEMACROS:rtty.mc>',
  '<AR> <BT> <KN> <SK>': '<CALL> DE <MYCALL> <KN>', '<WPM:n>': '<WPM:+2>', '<LANGUAGE:xx>': '<LANGUAGE:RU>', '<WK:comando>': '<WK:02>',
  '<LETTERS>': '<LETTERS>', '<NUMBERS>': '<NUMBERS>',
  '<LOADPICTURE>': '<LOADPICTURE>', '<FROMCLIP>': '<FROMCLIP>', '<FONT:nome>': '<FONT:Arial>', '<LIMITSCANLINES:n>': '<LIMITSCANLINES:128>',
  '<SET FILTER:n>': '<SET FILTER:2>', '<SET FONT:nome>': '<SET FONT:Feld>', '<SET SUBMODE:n>': '<SET SUBMODE:1>',
};

/** Altri nomi dello stesso comando, che Aurora capisce. */
const ALIASES: Record<string, readonly string[]> = {
  '<AFCON>': ['<AFCOFF>', '<AFC>'], '<SQUELCHON>': ['<SQUELCHOFF>', '<SQUELCH>'], '<SNAPON>': ['<SNAPOFF>', '<SNAP>'],
  '<LOCKTX>': ['<UNLOCKTX>', '<LOCKTXTOGGLE>'], '<STARTSCAN>': ['<STOPSCAN>'], '<MEMW:n>': ['<MEMR:n>'], '<SPLIT:1>': ['<SPLIT:0>'],
  '<QTCSEND>': ['<STATS:SendQTC>'], '<QTCREAD>': ['<STATS:ReadQTC>'],
  '<KHZ>': ['<KHZ:n>'], '<MHZ>': ['<MHZ:n>'], '<TUNE>': ['<TUNE:n>'],
};

/** La spiegazione lunga, per i comandi dove la breve non basta. */
const LONG: Record<string, readonly [string, string, string, string, string, string]> = {
  '<TX>': [
    'Il testo della macro (prima di un eventuale <RX>) parte via radio. Senza <RX> la trasmissione resta aperta finito il testo e quello che scrivi parte subito; con <RX> si torna in ricezione. <TX> da solo trasmette quello che c’è nella casella. Se stai già trasmettendo il testo si accoda. Aurora non apre mai una trasmissione vuota.',
    'The macro text (before any <RX>) goes out on the air. Without <RX> the transmission stays open after the text and what you type goes out at once; with <RX> it goes back to receive. <TX> alone sends what is in the box. If you are already transmitting, the text is queued. Aurora never opens an empty transmission.',
    'Der Makrotext (vor einem eventuellen <RX>) wird gesendet. Ohne <RX> bleibt die Sendung nach dem Text offen und was du tippst, geht sofort raus; mit <RX> geht es zurück zum Empfang. <TX> allein sendet den Text im Feld. Sendest du schon, wird der Text angehängt. Aurora öffnet nie eine leere Sendung.',
    'Le texte de la macro (avant un éventuel <RX>) part en l’air. Sans <RX> l’émission reste ouverte après le texte et ce que tu tapes part tout de suite ; avec <RX> on revient en réception. <TX> seul émet ce qui est dans la case. Si tu émets déjà, le texte est mis à la suite. Aurora n’ouvre jamais une émission vide.',
    'El texto de la macro (antes de un posible <RX>) sale al aire. Sin <RX> la transmisión sigue abierta tras el texto y lo que escribes sale enseguida; con <RX> vuelve a recepción. <TX> solo envía lo que hay en la casilla. Si ya estás transmitiendo, el texto se pone a la cola. Aurora nunca abre una transmisión vacía.',
    'O texto da macro (antes de um eventual <RX>) vai para o ar. Sem <RX> a transmissão fica aberta depois do texto e o que escreves sai logo; com <RX> volta à receção. <TX> sozinho envia o que está na caixa. Se já estás a transmitir, o texto fica em fila. O Aurora nunca abre uma transmissão vazia.',
  ],
  '<RX>': [
    '<RX> è un segno dentro il testo: quando la trasmissione arriva a quel punto si torna in ricezione. Il testo scritto dopo <RX> non parte: resta nella finestra di trasmissione, pronto per la prossima volta. Se la macro non trasmette e non stai trasmettendo, <RX> non fa niente.',
    '<RX> is a mark inside the text: when the transmission reaches that point it goes back to receive. Text written after <RX> is not sent: it stays in the transmit window, ready for next time. If the macro does not transmit and you are not transmitting, <RX> does nothing.',
    '<RX> ist eine Marke im Text: Erreicht die Sendung diese Stelle, geht es zurück zum Empfang. Text nach <RX> wird nicht gesendet: er bleibt im Sendefenster für das nächste Mal. Sendet das Makro nicht und sendest du gerade nicht, tut <RX> nichts.',
    '<RX> est une marque dans le texte : quand l’émission arrive à ce point, on revient en réception. Le texte écrit après <RX> ne part pas : il reste dans la fenêtre d’émission pour la fois suivante. Si la macro n’émet pas et que tu n’émets pas, <RX> ne fait rien.',
    '<RX> es una marca dentro del texto: cuando la transmisión llega a ese punto se vuelve a recepción. El texto escrito después de <RX> no se envía: queda en la ventana de transmisión para la próxima vez. Si la macro no transmite y no estás transmitiendo, <RX> no hace nada.',
    '<RX> é uma marca dentro do texto: quando a transmissão chega a esse ponto volta-se à receção. O texto escrito depois de <RX> não é enviado: fica na janela de transmissão para a próxima vez. Se a macro não transmite e não estás a transmitir, <RX> não faz nada.',
  ],
  '<RXANDCLEAR>': [
    'Come <RX>, ma quando si è tornati in ricezione la finestra di trasmissione si svuota: sia la casella sia il testo già partito. Se la macro non trasmette, pulisce subito.',
    'Like <RX>, but once back to receive the transmit window is emptied: both the box and the text already sent. If the macro does not transmit, it clears at once.',
    'Wie <RX>, aber nach der Rückkehr zum Empfang wird das Sendefenster geleert: Feld und bereits gesendeter Text. Sendet das Makro nicht, wird sofort geleert.',
    'Comme <RX>, mais de retour en réception la fenêtre d’émission est vidée : la case et le texte déjà émis. Si la macro n’émet pas, elle efface tout de suite.',
    'Como <RX>, pero de vuelta en recepción la ventana de transmisión se vacía: la casilla y el texto ya enviado. Si la macro no transmite, limpia enseguida.',
    'Como <RX>, mas de volta à receção a janela de transmissão esvazia-se: a caixa e o texto já enviado. Se a macro não transmite, limpa logo.',
  ],
  '<CLEARTXWINDOW>': [
    'Conta dove lo metti: all’inizio pulisce la finestra e poi il testo della macro ci entra pulito; dopo il testo lo cancella. Dopo un <RX> pulisce quando si torna in ricezione.',
    'Where you put it matters: at the start it clears the window and then the macro text goes in clean; after the text it erases it. After an <RX> it clears when back to receive.',
    'Die Stelle zählt: am Anfang leert er das Fenster und der Makrotext kommt sauber hinein; nach dem Text löscht er ihn. Nach einem <RX> leert er bei der Rückkehr zum Empfang.',
    'L’endroit compte : au début il vide la fenêtre puis le texte de la macro y entre propre ; après le texte il l’efface. Après un <RX> il vide au retour en réception.',
    'Importa dónde lo pones: al principio limpia la ventana y luego el texto de la macro entra limpio; después del texto lo borra. Tras un <RX> limpia al volver a recepción.',
    'Importa onde o pões: no início limpa a janela e depois o texto da macro entra limpo; depois do texto apaga-o. Depois de um <RX> limpa ao voltar à receção.',
  ],
  '<TXE>': [
    'Come <TX>, ma prima aspetta che il segnale sulla frequenza di ricezione cada (il corrispondente ha finito). Dopo 30 secondi trasmette comunque; Esc annulla l’attesa.',
    'Like <TX>, but first waits for the signal on the receive frequency to drop (the other station has finished). After 30 seconds it transmits anyway; Esc cancels the wait.',
    'Wie <TX>, wartet aber zuerst, bis das Signal auf der Empfangsfrequenz wegfällt (die Gegenstation ist fertig). Nach 30 Sekunden wird trotzdem gesendet; Esc bricht das Warten ab.',
    'Comme <TX>, mais attend d’abord que le signal sur la fréquence de réception tombe (le correspondant a fini). Après 30 secondes il émet quand même ; Échap annule l’attente.',
    'Como <TX>, pero antes espera a que caiga la señal en la frecuencia de recepción (el corresponsal ha terminado). Tras 30 segundos transmite igualmente; Esc cancela la espera.',
    'Como <TX>, mas primeiro espera que o sinal na frequência de receção caia (o correspondente terminou). Após 30 segundos transmite na mesma; Esc cancela a espera.',
  ],
  '<AUTOCQ>': [
    'Da solo accende o spegne l’Auto CQ, che ripete la macro AutoCqText (o F2) con una pausa fra una chiamata e l’altra (<ACQDELAY>). Se dopo <AUTOCQ> c’è un testo, quel testo diventa la chiamata ripetuta (i suoi comandi si riempiono a ogni giro). Esc due volte spegne l’Auto CQ.',
    'Alone it turns Auto CQ on or off; it repeats the AutoCqText macro (or F2) with a pause between calls (<ACQDELAY>). If a text follows <AUTOCQ>, that text becomes the repeated call (its commands are filled in each round). Pressing Esc twice turns Auto CQ off.',
    'Allein schaltet er Auto-CQ ein oder aus; er wiederholt das Makro AutoCqText (oder F2) mit einer Pause zwischen den Rufen (<ACQDELAY>). Folgt auf <AUTOCQ> ein Text, wird er zum wiederholten Ruf (seine Befehle werden jede Runde neu gefüllt). Zweimal Esc schaltet Auto-CQ aus.',
    'Seul il active ou coupe l’Auto CQ, qui répète la macro AutoCqText (ou F2) avec une pause entre les appels (<ACQDELAY>). Si un texte suit <AUTOCQ>, ce texte devient l’appel répété (ses commandes sont remplies à chaque tour). Échap deux fois coupe l’Auto CQ.',
    'Solo activa o apaga el Auto CQ, que repite la macro AutoCqText (o F2) con una pausa entre llamadas (<ACQDELAY>). Si tras <AUTOCQ> hay un texto, ese texto pasa a ser la llamada repetida (sus comandos se rellenan en cada vuelta). Esc dos veces apaga el Auto CQ.',
    'Sozinho liga ou desliga o Auto CQ, que repete a macro AutoCqText (ou F2) com uma pausa entre chamadas (<ACQDELAY>). Se depois de <AUTOCQ> houver um texto, esse texto passa a ser a chamada repetida (os seus comandos preenchem-se a cada volta). Esc duas vezes desliga o Auto CQ.',
  ],
  '<NRS>': [
    'Lo scambio da mandare in gara. Se il campo è vuoto Aurora usa il numero progressivo: ultimo numero mandato + 1, con gli zeri fino a tre cifre (001). Le cifre vanno sempre in chiaro (niente numeri tagliati come T per 0).',
    'The exchange to send in a contest. If the field is empty Aurora uses the serial number: last number sent + 1, padded with zeros to three digits (001). Digits are always sent in full (no cut numbers like T for 0).',
    'Der im Contest zu sendende Austausch. Ist das Feld leer, nimmt Aurora die laufende Nummer: zuletzt gesendete + 1, mit Nullen auf drei Stellen (001). Ziffern gehen immer ausgeschrieben raus (keine gekürzten Zahlen wie T für 0).',
    'L’échange à envoyer en concours. Si le champ est vide Aurora utilise le numéro de série : dernier numéro envoyé + 1, complété par des zéros sur trois chiffres (001). Les chiffres partent toujours en entier (pas de chiffres abrégés comme T pour 0).',
    'El intercambio a enviar en un concurso. Si el campo está vacío Aurora usa el número de serie: último enviado + 1, con ceros hasta tres cifras (001). Las cifras van siempre completas (sin números cortados como T por 0).',
    'A troca a enviar num concurso. Se o campo estiver vazio o Aurora usa o número de série: último enviado + 1, com zeros até três algarismos (001). Os algarismos vão sempre por extenso (sem números cortados como T para 0).',
  ],
  '<SAVEQSO>': [
    'Salva a log il QSO in corso con i dati di quel momento. Mettilo prima di <CLEARQSO>: i comandi si eseguono nell’ordine in cui li scrivi.',
    'Saves the current QSO to the log with the data of that moment. Put it before <CLEARQSO>: commands run in the order you write them.',
    'Speichert das laufende QSO mit den aktuellen Daten im Log. Setze es vor <CLEARQSO>: die Befehle laufen in der Reihenfolge, in der du sie schreibst.',
    'Enregistre le QSO en cours au journal avec les données du moment. Place-le avant <CLEARQSO> : les commandes s’exécutent dans l’ordre où tu les écris.',
    'Guarda el QSO en curso en el log con los datos de ese momento. Ponlo antes de <CLEARQSO>: los comandos se ejecutan en el orden en que los escribes.',
    'Guarda o QSO em curso no log com os dados desse momento. Põe-no antes de <CLEARQSO>: os comandos executam-se pela ordem em que os escreves.',
  ],
  '<QTCSEND>': [
    'Solo con un contest WAE attivo. La finestra propone i QSO del contest che si possono ancora mandare alla stazione del QSO (non i suoi, non quelli già mandati, al massimo 10 per stazione) con il numero della serie; i tasti mandano l’intestazione «QTC n/m», tutte le righe o una sola (anche per ripeterla). Le righe confermate si salvano come mandate. Va bene anche il nome <STATS:SendQTC> dei file importati.',
    'Only with a WAE contest active. The window offers the contest QSOs that can still be sent to the station of the QSO (not its own, not those already sent, at most 10 per station) with the series number; the buttons send the «QTC n/m» header, all lines or just one (also to repeat it). Confirmed lines are saved as sent. The name <STATS:SendQTC> of imported files works too.',
    'Nur mit aktivem WAE-Contest. Das Fenster schlägt die Contest-QSOs vor, die der Station des QSO noch gesendet werden können (nicht ihre eigenen, nicht schon gesendete, höchstens 10 pro Station), mit der Seriennummer; die Tasten senden den Kopf «QTC n/m», alle Zeilen oder eine einzelne (auch zum Wiederholen). Bestätigte Zeilen werden als gesendet gespeichert. Auch der Name <STATS:SendQTC> importierter Dateien geht.',
    'Seulement avec un concours WAE actif. La fenêtre propose les QSO du concours qu’on peut encore envoyer à la station du QSO (pas les siens, pas ceux déjà envoyés, 10 au plus par station) avec le numéro de série ; les boutons envoient l’en-tête « QTC n/m », toutes les lignes ou une seule (aussi pour la répéter). Les lignes confirmées sont enregistrées comme envoyées. Le nom <STATS:SendQTC> des fichiers importés marche aussi.',
    'Solo con un concurso WAE activo. La ventana propone los QSO del concurso que aún se pueden enviar a la estación del QSO (no los suyos, no los ya enviados, como mucho 10 por estación) con el número de serie; los botones envían la cabecera «QTC n/m», todas las líneas o una sola (también para repetirla). Las líneas confirmadas se guardan como enviadas. También vale el nombre <STATS:SendQTC> de los archivos importados.',
    'Só com um concurso WAE ativo. A janela propõe os QSO do concurso que ainda se podem enviar à estação do QSO (não os seus, não os já enviados, no máximo 10 por estação) com o número da série; os botões enviam o cabeçalho «QTC n/m», todas as linhas ou só uma (também para a repetir). As linhas confirmadas guardam-se como enviadas. Também serve o nome <STATS:SendQTC> dos ficheiros importados.',
  ],
  '<QTCREAD>': [
    'Solo con un contest WAE attivo. «QRV» dice che sei pronto, «Prendi dalla ricezione» legge dalla finestra RX l’intestazione «QTC n/m» e le righe ora-nominativo-numero, che puoi correggere; «AGN» chiede di ripetere una riga, tutte o l’intestazione. «Salva» registra la serie e manda la conferma. Va bene anche il nome <STATS:ReadQTC> dei file importati.',
    'Only with a WAE contest active. «QRV» says you are ready, «Take from receive» reads the «QTC n/m» header and the time-call-number lines from the RX window, and you can correct them; «AGN» asks to repeat a line, all of them or the header. «Save» stores the series and sends the confirmation. The name <STATS:ReadQTC> of imported files works too.',
    'Nur mit aktivem WAE-Contest. «QRV» meldet Bereitschaft, «Aus dem Empfang» liest den Kopf «QTC n/m» und die Zeilen Zeit-Rufzeichen-Nummer aus dem RX-Fenster, die du korrigieren kannst; «AGN» fordert eine Zeile, alle oder den Kopf erneut an. «Speichern» legt die Serie ab und sendet die Bestätigung. Auch der Name <STATS:ReadQTC> importierter Dateien geht.',
    'Seulement avec un concours WAE actif. « QRV » dit que tu es prêt, « Prendre de la réception » lit dans la fenêtre RX l’en-tête « QTC n/m » et les lignes heure-indicatif-numéro, que tu peux corriger ; « AGN » demande de répéter une ligne, toutes ou l’en-tête. « Enregistrer » garde la série et envoie la confirmation. Le nom <STATS:ReadQTC> des fichiers importés marche aussi.',
    'Solo con un concurso WAE activo. «QRV» dice que estás listo, «Tomar de la recepción» lee de la ventana RX la cabecera «QTC n/m» y las líneas hora-indicativo-número, que puedes corregir; «AGN» pide repetir una línea, todas o la cabecera. «Guardar» registra la serie y envía la confirmación. También vale el nombre <STATS:ReadQTC> de los archivos importados.',
    'Só com um concurso WAE ativo. «QRV» diz que estás pronto, «Tirar da receção» lê da janela RX o cabeçalho «QTC n/m» e as linhas hora-indicativo-número, que podes corrigir; «AGN» pede para repetir uma linha, todas ou o cabeçalho. «Guardar» regista a série e envia a confirmação. Também serve o nome <STATS:ReadQTC> dos ficheiros importados.',
  ],
  '<AR> <BT> <KN> <SK>': [
    'In CW ognuno è un segno solo, senza pausa fra le lettere. In RTTY e PSK escono come lettere normali (AR, BT, KN, SK).',
    'In CW each is a single sign, with no gap between the letters. In RTTY and PSK they go out as plain letters (AR, BT, KN, SK).',
    'In CW ist jedes ein einziges Zeichen ohne Pause zwischen den Buchstaben. In RTTY und PSK gehen sie als normale Buchstaben raus (AR, BT, KN, SK).',
    'En CW chacun est un seul signe, sans espace entre les lettres. En RTTY et PSK ils partent comme des lettres normales (AR, BT, KN, SK).',
    'En CW cada uno es un solo signo, sin pausa entre las letras. En RTTY y PSK salen como letras normales (AR, BT, KN, SK).',
    'Em CW cada um é um só sinal, sem pausa entre as letras. Em RTTY e PSK saem como letras normais (AR, BT, KN, SK).',
  ],
  '<FQ:kHz>': [
    'La frequenza è quella del segnale: Aurora sposta la radio in modo che il segnale cada sulla frequenza audio di ricezione attuale.',
    'The frequency is that of the signal: Aurora moves the radio so the signal falls on the current receive audio frequency.',
    'Die Frequenz ist die des Signals: Aurora verstellt das Funkgerät so, dass das Signal auf die aktuelle Empfangs-Audiofrequenz fällt.',
    'La fréquence est celle du signal : Aurora déplace la radio pour que le signal tombe sur la fréquence audio de réception actuelle.',
    'La frecuencia es la de la señal: Aurora mueve la radio para que la señal caiga en la frecuencia de audio de recepción actual.',
    'A frequência é a do sinal: o Aurora move o rádio para que o sinal caia na frequência de áudio de receção atual.',
  ],
  '<TEXTFILE:nome>': [
    'Il file si cerca nella cartella dei testi delle macro di Aurora; se non c’è il comando non scrive niente.',
    'The file is looked up in Aurora’s macro texts folder; if it is missing the command writes nothing.',
    'Die Datei wird im Ordner der Makrotexte von Aurora gesucht; fehlt sie, schreibt der Befehl nichts.',
    'Le fichier est cherché dans le dossier des textes des macros d’Aurora ; s’il manque, la commande n’écrit rien.',
    'El archivo se busca en la carpeta de textos de las macros de Aurora; si no está, el comando no escribe nada.',
    'O ficheiro procura-se na pasta dos textos das macros do Aurora; se não existir, o comando não escreve nada.',
  ],
  '<ROTOR:SP>': [
    'La direzione si calcola dal tuo locatore e da quello del corrispondente nel QSO in corso; senza uno dei due Aurora lo dice e non gira.',
    'The heading is worked out from your locator and the other station’s one in the current QSO; without either, Aurora says so and does not turn.',
    'Die Richtung wird aus deinem Locator und dem der Gegenstation im laufenden QSO berechnet; fehlt einer, sagt Aurora es und dreht nicht.',
    'Le cap est calculé à partir de ton locator et de celui du correspondant du QSO en cours ; sans l’un des deux, Aurora le dit et ne tourne pas.',
    'El rumbo se calcula con tu locator y el del corresponsal del QSO en curso; sin uno de los dos Aurora lo avisa y no gira.',
    'O rumo calcula-se com o teu locator e o do correspondente do QSO em curso; sem um dos dois o Aurora avisa e não roda.',
  ],
};

/** Le macro evento: non legate a un tasto, partono da sole quando succede qualcosa. `runs`: Aurora le esegue. */
export interface MacroEvent {
  name: string;
  desc: readonly [string, string, string, string, string, string];
  runs: boolean;
}

export const MACRO_EVENTS: readonly MacroEvent[] = [
  { name: 'OnStart', runs: true, desc: ['all’apertura della pagina dei modi testuali', 'when the text-modes page opens', 'beim Öffnen der Seite der Textmodi', 'à l’ouverture de la page des modes texte', 'al abrir la página de los modos de texto', 'ao abrir a página dos modos de texto'] },
  { name: 'OnEnd', runs: true, desc: ['alla chiusura della pagina (se si va in un’altra pagina o si chiude Aurora)', 'when the page closes (going to another page or closing Aurora)', 'beim Schließen der Seite (Wechsel zu einer anderen Seite oder Beenden von Aurora)', 'à la fermeture de la page (autre page ou fermeture d’Aurora)', 'al cerrar la página (otra página o cierre de Aurora)', 'ao fechar a página (outra página ou fecho do Aurora)'] },
  { name: 'OnTX', runs: true, desc: ['quando parte una trasmissione', 'when a transmission starts', 'wenn eine Sendung beginnt', 'quand une émission commence', 'cuando empieza una transmisión', 'quando começa uma transmissão'] },
  { name: 'OnCR', runs: true, desc: ['con Invio nella casella di trasmissione', 'on Enter in the transmit box', 'bei Enter im Sendefeld', 'avec Entrée dans la case d’émission', 'con Intro en la casilla de transmisión', 'com Enter na caixa de transmissão'] },
  { name: 'OnClickAny', runs: true, desc: ['quando una parola della ricezione va in un campo del log (doppio clic o menu del tasto destro)', 'when a word from the receive window goes into a log field (double click or right-click menu)', 'wenn ein Wort aus dem Empfang in ein Logfeld kommt (Doppelklick oder Rechtsklickmenü)', 'quand un mot de la réception va dans un champ du log (double clic ou menu du clic droit)', 'cuando una palabra de la recepción va a un campo del log (doble clic o menú del botón derecho)', 'quando uma palavra da receção vai para um campo do log (duplo clique ou menu do botão direito)'] },
  { name: 'Gray+', runs: true, desc: ['con il tasto + del tastierino numerico', 'with the + key of the numeric keypad', 'mit der Taste + des Ziffernblocks', 'avec la touche + du pavé numérique', 'con la tecla + del teclado numérico', 'com a tecla + do teclado numérico'] },
  { name: 'AutoCqText', runs: true, desc: ['il testo che l’Auto CQ ripete (se manca, quello di F2)', 'the text Auto CQ repeats (if missing, that of F2)', 'der Text, den Auto-CQ wiederholt (fehlt er, der von F2)', 'le texte que l’Auto CQ répète (s’il manque, celui de F2)', 'el texto que repite el Auto CQ (si falta, el de F2)', 'o texto que o Auto CQ repete (se faltar, o de F2)'] },
  { name: 'OnAlarm', runs: false, desc: ['avviso sul testo ricevuto: Aurora non ha questo avviso, la macro non parte', 'alert on received text: Aurora has no such alert, the macro does not run', 'Alarm auf empfangenen Text: Aurora hat diesen Alarm nicht, das Makro startet nicht', 'alerte sur le texte reçu : Aurora n’a pas cette alerte, la macro ne part pas', 'aviso sobre el texto recibido: Aurora no tiene este aviso, la macro no se ejecuta', 'aviso sobre o texto recebido: o Aurora não tem este aviso, a macro não arranca'] },
  { name: 'OnSP0', runs: false, desc: ['non ha un evento corrispondente in Aurora: la macro non parte', 'has no matching event in Aurora: the macro does not run', 'hat kein passendes Ereignis in Aurora: das Makro startet nicht', 'n’a pas d’événement correspondant dans Aurora : la macro ne part pas', 'no tiene un evento correspondiente en Aurora: la macro no se ejecuta', 'não tem evento correspondente no Aurora: a macro não arranca'] },
  { name: 'OnSP1', runs: false, desc: ['non ha un evento corrispondente in Aurora: la macro non parte', 'has no matching event in Aurora: the macro does not run', 'hat kein passendes Ereignis in Aurora: das Makro startet nicht', 'n’a pas d’événement correspondant dans Aurora : la macro ne part pas', 'no tiene un evento correspondiente en Aurora: la macro no se ejecuta', 'não tem evento correspondente no Aurora: a macro não arranca'] },
  { name: 'Hello', runs: false, desc: ['non ha un evento corrispondente in Aurora: si può lanciare con <EDITMACRO:Hello> per modificarla, ma non parte da sola', 'has no matching event in Aurora: you can open it with <EDITMACRO:Hello> to edit it, but it does not run by itself', 'hat kein passendes Ereignis in Aurora: mit <EDITMACRO:Hello> lässt es sich bearbeiten, startet aber nicht von selbst', 'n’a pas d’événement correspondant dans Aurora : on peut l’ouvrir avec <EDITMACRO:Hello> pour la modifier, mais elle ne part pas seule', 'no tiene un evento correspondiente en Aurora: se puede abrir con <EDITMACRO:Hello> para editarla, pero no se ejecuta sola', 'não tem evento correspondente no Aurora: pode abrir-se com <EDITMACRO:Hello> para a editar, mas não arranca sozinha'] },
];

export const MACRO_COMMANDS: readonly MacroCommand[] = RAW_COMMANDS.map((r) => ({
  ...r,
  cat: CATEGORY[r.tag] ?? 'data',
  ex: EXAMPLE[r.tag] ?? r.tag,
  ...(LONG[r.tag] ? { long: LONG[r.tag] } : {}),
  ...(ALIASES[r.tag] ? { aliases: ALIASES[r.tag] } : {}),
}));

/** Per i controlli: i comandi senza categoria o senza esempio scritti a mano. */
export const UNMAPPED = RAW_COMMANDS.filter((r) => !CATEGORY[r.tag] || !EXAMPLE[r.tag]).map((r) => r.tag);

/** Le voci di un gruppo (tutte se `group` e' vuoto). */
export function commandsOf(group: CommandGroup | ''): readonly MacroCommand[] {
  return group === '' ? MACRO_COMMANDS : MACRO_COMMANDS.filter((x) => x.group === group);
}
