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
  c('text', '<PREVCALL>', 'contest: il nominativo dell’ultimo QSO del log', 'contest: the callsign of the last logged QSO', 'Contest: das Rufzeichen des letzten geloggten QSO', 'contest : l’indicatif du dernier QSO du log', 'contest: el indicativo del último QSO del log', 'contest: o indicativo do último QSO do log'),
  c('text', '<PREVRSTS>', 'contest: il rapporto mandato nell’ultimo QSO del log', 'contest: the report sent in the last logged QSO', 'Contest: der im letzten geloggten QSO gesendete Rapport', 'contest : le report envoyé dans le dernier QSO du log', 'contest: el reporte enviado en el último QSO del log', 'contest: o relatório enviado no último QSO do log'),
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
  c('program', '<TUNE>', "portante per accordare sulla frequenza di trasmissione (<TUNE:s> per s secondi, di base 10)", "tuning carrier on the transmit frequency (<TUNE:s> for s seconds, 10 by default)", "Abstimmträger auf der Sendefrequenz (<TUNE:s> für s Sekunden, standardmäßig 10)", "porteuse d'accord sur la fréquence d'émission (<TUNE:s> pour s secondes, 10 par défaut)", "portadora de sintonía en la frecuencia de transmisión (<TUNE:s> para s segundos, 10 por defecto)", "portadora de sintonia na frequência de transmissão (<TUNE:s> durante s segundos, 10 por predefinição)"),
  c('program', '<BEEP:ON>', 'suona un avviso', 'sounds a beep', 'gibt einen Signalton aus', 'émet un bip', 'emite un pitido', 'emite um aviso sonoro'),
  c('program', '<CURSOR:TX>', 'porta il cursore nella casella di trasmissione', 'puts the cursor in the transmit box', 'setzt den Cursor ins Sendefeld', 'place le curseur dans la case d’émission', 'pone el cursor en la casilla de transmisión', 'põe o cursor na caixa de transmissão'),
  c('macros', '<MACROS:file>', "Aurora tiene le macro nelle sue impostazioni: per caricarne un file usa Importa macro; il comando lo ricorda", "Aurora keeps macros in its settings: to load a file use Import macros; the command reminds you", "Aurora bewahrt die Makros in ihren Einstellungen auf: zum Laden einer Datei nutze Makros importieren; der Befehl erinnert dich daran", "Aurora garde les macros dans ses paramètres : pour charger un fichier utilise Importer des macros ; la commande te le rappelle", "Aurora guarda las macros en sus ajustes: para cargar un archivo usa Importar macros; el comando te lo recuerda", "O Aurora guarda as macros nas suas definições: para carregar um ficheiro usa Importar macros; o comando lembra-te disso"),
  c('frequency', '<AUDIOFQ:Hz>', 'imposta la frequenza audio (con + o - è relativa)', 'sets the audio frequency (relative with + or -)', 'setzt die Audiofrequenz (mit + oder - relativ)', 'règle la fréquence audio (relative avec + ou -)', 'fija la frecuencia de audio (relativa con + o -)', 'define a frequência de áudio (relativa com + ou -)'),
  c('frequency', '<SEEKLEFT>', 'cerca un segnale verso sinistra', 'seeks a signal to the left', 'sucht ein Signal nach links', 'cherche un signal vers la gauche', 'busca una señal hacia la izquierda', 'procura um sinal para a esquerda'),
  c('frequency', '<SEEKRIGHT>', 'cerca un segnale verso destra', 'seeks a signal to the right', 'sucht ein Signal nach rechts', 'cherche un signal vers la droite', 'busca una señal hacia la derecha', 'procura um sinal para a direita'),
  c('frequency', '<BOOKMARK>', "segna ora e frequenza nella finestra RX (come <MARK>)", "marks time and frequency in the RX window (like <MARK>)", "markiert Zeit und Frequenz im RX-Fenster (wie <MARK>)", "marque l'heure et la fréquence dans la fenêtre RX (comme <MARK>)", "marca la hora y la frecuencia en la ventana RX (como <MARK>)", "marca a hora e a frequência na janela RX (como <MARK>)"),
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
  c('macros', '<MODEMACROS:file>', "usa le macro di un modo (PSK31, PSK63, RTTY, CW) senza cambiare modo; vuoto: quelle del modo in uso", "uses the macros of a mode (PSK31, PSK63, RTTY, CW) without changing mode; empty: those of the current mode", "nutzt die Makros eines Modus (PSK31, PSK63, RTTY, CW), ohne den Modus zu wechseln; leer: die des aktuellen Modus", "utilise les macros d'un mode (PSK31, PSK63, RTTY, CW) sans changer de mode ; vide : celles du mode actuel", "usa las macros de un modo (PSK31, PSK63, RTTY, CW) sin cambiar de modo; vacío: las del modo actual", "usa as macros de um modo (PSK31, PSK63, RTTY, CW) sem mudar de modo; vazio: as do modo atual"),
  c('macros', '<HIDETEXT>', "il testo che segue non si trasmette fino a <SHOWTEXT> (erano comandi per un TNC esterno)", "the following text is not transmitted until <SHOWTEXT> (they were commands for an external TNC)", "der folgende Text wird bis <SHOWTEXT> nicht gesendet (es waren Befehle für einen externen TNC)", "le texte suivant n'est pas émis jusqu'à <SHOWTEXT> (c'étaient des commandes pour un TNC externe)", "el texto siguiente no se transmite hasta <SHOWTEXT> (eran comandos para un TNC externo)", "o texto seguinte não é transmitido até <SHOWTEXT> (eram comandos para um TNC externo)"),
  c('macros', '<SHOWTEXT>', "il testo torna a essere trasmesso dopo <HIDETEXT>", "text is transmitted again after <HIDETEXT>", "Text wird nach <HIDETEXT> wieder gesendet", "le texte est de nouveau émis après <HIDETEXT>", "el texto se vuelve a transmitir después de <HIDETEXT>", "o texto volta a ser transmitido depois de <HIDETEXT>"),
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
  c('rtty', '<LETTERS>', "RTTY: il passaggio alle lettere lo manda già il motore dove serve, il comando non aggiunge niente", "RTTY: the engine already sends the letters shift where needed, the command adds nothing", "RTTY: die Engine sendet die Buchstabenumschaltung schon, wo nötig, der Befehl fügt nichts hinzu", "RTTY : le moteur envoie déjà le passage en lettres là où il faut, la commande n'ajoute rien", "RTTY: el motor ya envía el cambio a letras donde hace falta, el comando no añade nada", "RTTY: o motor já envia a mudança para letras onde é preciso, o comando não acrescenta nada"),
  c('rtty', '<NUMBERS>', "RTTY: il passaggio alle cifre lo manda già il motore dove serve, il comando non aggiunge niente", "RTTY: the engine already sends the figures shift where needed, the command adds nothing", "RTTY: die Engine sendet die Ziffernumschaltung schon, wo nötig, der Befehl fügt nichts hinzu", "RTTY : le moteur envoie déjà le passage en chiffres là où il faut, la commande n'ajoute rien", "RTTY: el motor ya envía el cambio a cifras donde hace falta, el comando no añade nada", "RTTY: o motor já envia a mudança para algarismos onde é preciso, o comando não acrescenta nada"),
  // SSTV
  c('sstv', '<LOADPICTURE>', "apre la pagina SSTV per scegliere l'immagine da trasmettere", "opens the SSTV page to choose the picture to send", "öffnet die SSTV-Seite zur Auswahl des zu sendenden Bildes", "ouvre la page SSTV pour choisir l'image à envoyer", "abre la página de SSTV para elegir la imagen que se enviará", "abre a página de SSTV para escolher a imagem a enviar"),
  c('sstv', '<FROMCLIP>', "trasmette in SSTV l'immagine che c'è negli appunti", "transmits in SSTV the picture in the clipboard", "sendet in SSTV das Bild aus der Zwischenablage", "émet en SSTV l'image du presse-papiers", "transmite en SSTV la imagen del portapapeles", "transmite em SSTV a imagem da área de transferência"),
  c('sstv', '<FONT:nome>', "i testi sull'immagine si scrivono nella pagina SSTV: il comando la apre", "texts on the picture are written in the SSTV page: the command opens it", "Texte auf dem Bild werden auf der SSTV-Seite geschrieben: der Befehl öffnet sie", "les textes sur l'image s'écrivent dans la page SSTV : la commande l'ouvre", "los textos de la imagen se escriben en la página de SSTV: el comando la abre", "os textos na imagem escrevem-se na página de SSTV: o comando abre-a"),
  c('sstv', '<LIMITSCANLINES:n>', "le righe dell'immagine si scelgono nella pagina SSTV: il comando la apre", "picture lines are chosen in the SSTV page: the command opens it", "Bildzeilen werden auf der SSTV-Seite gewählt: der Befehl öffnet sie", "les lignes de l'image se choisissent dans la page SSTV : la commande l'ouvre", "las líneas de la imagen se eligen en la página de SSTV: el comando la abre", "as linhas da imagem escolhem-se na página de SSTV: o comando abre-a"),
  // CW
  c('cw', '<AR> <BT> <KN> <SK>', 'segni di procedura: in CW un segno solo, negli altri modi le lettere', 'procedure signs: one sign in CW, the letters in other modes', 'Betriebszeichen: in CW ein Zeichen, in anderen Modi die Buchstaben', 'signes de procédure : un seul signe en CW, les lettres dans les autres modes', 'signos de procedimiento: un solo signo en CW, las letras en los otros modos', 'sinais de procedimento: um só sinal em CW, as letras nos outros modos'),
  c('cw', '<WPM:n>', 'velocità in parole al minuto (con + o - è relativa)', 'speed in words per minute (relative with + or -)', 'Geschwindigkeit in Wörtern pro Minute (mit + oder - relativ)', 'vitesse en mots par minute (relative avec + ou -)', 'velocidad en palabras por minuto (relativa con + o -)', 'velocidade em palavras por minuto (relativa com + ou -)'),
  c('cw', '<LANGUAGE:xx>', "lingua delle frasi INT (come <FORCELANGUAGE:xx>)", "language of the INT phrases (like <FORCELANGUAGE:xx>)", "Sprache der INT-Sätze (wie <FORCELANGUAGE:xx>)", "langue des phrases INT (comme <FORCELANGUAGE:xx>)", "idioma de las frases INT (como <FORCELANGUAGE:xx>)", "língua das frases INT (como <FORCELANGUAGE:xx>)"),
  c('cw', '<WK:comando>', "manda il testo in CW con il WinKeyer (come <WKTEXT>)", "sends the text in CW with the WinKeyer (like <WKTEXT>)", "sendet den Text in CW mit dem WinKeyer (wie <WKTEXT>)", "envoie le texte en CW avec le WinKeyer (comme <WKTEXT>)", "envía el texto en CW con el WinKeyer (como <WKTEXT>)", "envia o texto em CW com o WinKeyer (como <WKTEXT>)"),
  // HELL
  c('hell', '<SET FILTER:n>', "Hell non c'è in Aurora: parametro saltato", "Aurora has no Hell: parameter skipped", "Aurora hat kein Hell: Parameter übersprungen", "Aurora n'a pas de Hell : paramètre ignoré", "Aurora no tiene Hell: parámetro omitido", "O Aurora não tem Hell: parâmetro ignorado"),
  c('hell', '<SET FONT:nome>', "Hell non c'è in Aurora: parametro saltato", "Aurora has no Hell: parameter skipped", "Aurora hat kein Hell: Parameter übersprungen", "Aurora n'a pas de Hell : paramètre ignoré", "Aurora no tiene Hell: parámetro omitido", "O Aurora não tem Hell: parâmetro ignorado"),
  c('hell', '<SET SUBMODE:n>', "sottomodo: BPSK31, PSK63, RTTY, CW cambiano modo; quelli che Aurora non ha (Hell, MFSK) si saltano", "submode: BPSK31, PSK63, RTTY, CW change mode; those Aurora lacks (Hell, MFSK) are skipped", "Submodus: BPSK31, PSK63, RTTY, CW wechseln den Modus; die, die Aurora nicht hat (Hell, MFSK), werden übersprungen", "sous-mode : BPSK31, PSK63, RTTY, CW changent de mode ; ceux qu'Aurora n'a pas (Hell, MFSK) sont ignorés", "submodo: BPSK31, PSK63, RTTY, CW cambian de modo; los que Aurora no tiene (Hell, MFSK) se omiten", "submodo: BPSK31, PSK63, RTTY, CW mudam de modo; os que o Aurora não tem (Hell, MFSK) são ignorados"),
  c('text', "<ACALL>", "nominativo del corrispondente senza /P, /M e prefissi", "the other station's callsign without /P, /M and prefixes", "Rufzeichen der Gegenstation ohne /P, /M und Präfixe", "l'indicatif de la station correspondante sans /P, /M ni préfixes", "el indicativo de la estación corresponsal sin /P, /M ni prefijos", "o indicativo da estação correspondente sem /P, /M e prefixos"),
  c('text', "<CALLPOST:testo>", "il nominativo seguito dal testo, solo se il nominativo c'è", "the callsign followed by the text, only if there is a callsign", "das Rufzeichen gefolgt vom Text, nur wenn ein Rufzeichen vorhanden ist", "l'indicatif suivi du texte, seulement s'il y a un indicatif", "el indicativo seguido del texto, solo si hay indicativo", "o indicativo seguido do texto, só se houver indicativo"),
  c('text', "<POST:sì,no>", "il primo testo se c'è un nominativo, il secondo se manca", "the first text if there is a callsign, the second if not", "der erste Text, wenn ein Rufzeichen vorhanden ist, der zweite, wenn nicht", "le premier texte s'il y a un indicatif, le second sinon", "el primer texto si hay indicativo, el segundo si no", "o primeiro texto se houver indicativo, o segundo se não houver"),
  c('text', "<OM>", "OM o YL secondo il nome del corrispondente", "OM or YL from the other operator's name", "OM oder YL nach dem Namen des Funkpartners", "OM ou YL selon le prénom du correspondant", "OM o YL según el nombre del corresponsal", "OM ou YL conforme o nome do correspondente"),
  c('text', "<DIST>", "distanza in km dal tuo locatore a quello del corrispondente (? se manca)", "distance in km from your locator to the other station's (? if missing)", "Entfernung in km von deinem Locator zu dem der Gegenstation (? wenn unbekannt)", "distance en km de ton locator à celui de la station correspondante (? s'il manque)", "distancia en km desde tu locator al de la estación corresponsal (? si falta)", "distância em km do teu locator ao da estação correspondente (? se faltar)"),
  c('text', "<MILES>", "come <DIST>, in miglia", "like <DIST>, in miles", "wie <DIST>, in Meilen", "comme <DIST>, en miles", "como <DIST>, en millas", "como <DIST>, em milhas"),
  c('text', "<AZ>", "direzione in gradi verso il corrispondente (? se manca il locatore)", "heading in degrees to the other station (? without a locator)", "Richtung in Grad zur Gegenstation (? ohne Locator)", "direction en degrés vers la station correspondante (? sans locator)", "rumbo en grados hacia la estación corresponsal (? sin locator)", "direção em graus para a estação correspondente (? sem locator)"),
  c('text', "<PREVNRR>", "lo scambio ricevuto nell'ultimo QSO del log", "the exchange received in the last logged QSO", "der Austausch, der im letzten geloggten QSO empfangen wurde", "l'échange reçu dans le dernier QSO du log", "el intercambio recibido en el último QSO del log", "a troca recebida no último QSO do log"),
  c('text', "<PREVCCALL>", "il nominativo dell'ultimo QSO senza /P e prefissi", "the callsign of the last QSO without /P and prefixes", "das Rufzeichen des letzten QSO ohne /P und Präfixe", "l'indicatif du dernier QSO sans /P ni préfixes", "el indicativo del último QSO sin /P ni prefijos", "o indicativo do último QSO sem /P e prefixos"),
  c('text', "<PREVNAME>", "il nome dell'ultimo QSO del log", "the name in the last logged QSO", "der Name im letzten geloggten QSO", "le nom du dernier QSO du log", "el nombre del último QSO del log", "o nome no último QSO do log"),
  c('text', "<PREVQTH>", "il QTH dell'ultimo QSO del log", "the QTH in the last logged QSO", "das QTH im letzten geloggten QSO", "le QTH du dernier QSO du log", "el QTH del último QSO del log", "o QTH no último QSO do log"),
  c('text', "<PREVRSTR>", "il rapporto ricevuto nell'ultimo QSO del log", "the report received in the last logged QSO", "der Rapport, der im letzten geloggten QSO empfangen wurde", "le report reçu dans le dernier QSO du log", "el reporte recibido en el último QSO del log", "o relatório recebido no último QSO do log"),
  c('text', "<PREVCTIME>", "ora UTC (HHMM) dell'ultimo QSO del log", "UTC time (HHMM) of the last logged QSO", "UTC-Zeit (HHMM) des letzten geloggten QSO", "heure UTC (HHMM) du dernier QSO du log", "hora UTC (HHMM) del último QSO del log", "hora UTC (HHMM) do último QSO do log"),
  c('text', "<PREVSUF2>", "le ultime due lettere del nominativo dell'ultimo QSO", "the last two letters of the last QSO's callsign", "die letzten zwei Buchstaben des Rufzeichens des letzten QSO", "les deux dernières lettres de l'indicatif du dernier QSO", "las dos últimas letras del indicativo del último QSO", "as duas últimas letras do indicativo do último QSO"),
  c('text', "<LASTCALL>", "rimette nel QSO il nominativo dell'ultimo QSO del log", "puts the callsign of the last logged QSO back into the QSO", "übernimmt das Rufzeichen des letzten geloggten QSO wieder ins QSO", "remet dans le QSO l'indicatif du dernier QSO du log", "vuelve a poner en el QSO el indicativo del último QSO del log", "volta a pôr no QSO o indicativo do último QSO do log"),
  c('text', "<NRRPOST:testo>", "lo scambio ricevuto con i numeri abbreviati della CW tradotti (T e O = 0, A = 1, N = 9), poi il testo", "the received exchange with CW cut numbers turned back into digits (T and O = 0, A = 1, N = 9), then the text", "der empfangene Austausch mit zurück in Ziffern verwandelten CW-Kurzzahlen (T und O = 0, A = 1, N = 9), dann der Text", "l'échange reçu avec les chiffres abrégés CW redevenus des chiffres (T et O = 0, A = 1, N = 9), puis le texte", "el intercambio recibido con los números abreviados de CW convertidos de nuevo en cifras (T y O = 0, A = 1, N = 9), y luego el texto", "a troca recebida com os números abreviados de CW convertidos de novo em algarismos (T e O = 0, A = 1, N = 9), depois o texto"),
  c('text', "<QSOBEFORE>", "data e ora del QSO già fatto con questo nominativo sulla stessa banda e nello stesso modo (vuoto se non c'è)", "date and time of the earlier QSO with this callsign on the same band and mode (empty if none)", "Datum und Uhrzeit des früheren QSO mit diesem Rufzeichen auf demselben Band und in demselben Modus (leer, wenn keins)", "date et heure du QSO précédent avec cet indicatif sur la même bande et dans le même mode (vide s'il n'y en a pas)", "fecha y hora del QSO anterior con este indicativo en la misma banda y modo (vacío si no hay)", "data e hora do QSO anterior com este indicativo na mesma banda e modo (vazio se não houver)"),
  c('text', "<QSOBEFORETIME1>", "ora UTC (HHMM) del QSO già fatto con questo nominativo", "UTC time (HHMM) of the earlier QSO with this callsign", "UTC-Zeit (HHMM) des früheren QSO mit diesem Rufzeichen", "heure UTC (HHMM) du QSO précédent avec cet indicatif", "hora UTC (HHMM) del QSO anterior con este indicativo", "hora UTC (HHMM) do QSO anterior com este indicativo"),
  c('text', "<QSOBEFOREBAND>", "banda del QSO già fatto con questo nominativo", "band of the earlier QSO with this callsign", "Band des früheren QSO mit diesem Rufzeichen", "bande du QSO précédent avec cet indicatif", "banda del QSO anterior con este indicativo", "banda do QSO anterior com este indicativo"),
  c('text', "<QSOBEFOREMODE>", "modo del QSO già fatto con questo nominativo", "mode of the earlier QSO with this callsign", "Modus des früheren QSO mit diesem Rufzeichen", "mode du QSO précédent avec cet indicatif", "modo del QSO anterior con este indicativo", "modo do QSO anterior com este indicativo"),
  c('text', "<ONQSOBEFORE:macro>", "se il nominativo è già stato lavorato esegue la macro indicata", "runs the given macro if the callsign was already worked", "startet das angegebene Makro, wenn das Rufzeichen schon gearbeitet wurde", "exécute la macro indiquée si l'indicatif a déjà été contacté", "ejecuta la macro indicada si el indicativo ya se trabajó", "executa a macro indicada se o indicativo já foi trabalhado"),
  c('text', "<SELCALL>", "la parola selezionata nella finestra RX diventa il nominativo del QSO", "the word selected in the RX window becomes the QSO callsign", "das im RX-Fenster ausgewählte Wort wird zum Rufzeichen des QSO", "le mot sélectionné dans la fenêtre RX devient l'indicatif du QSO", "la palabra seleccionada en la ventana RX pasa a ser el indicativo del QSO", "a palavra selecionada na janela RX passa a ser o indicativo do QSO"),
  c('text', "<SELNAME>", "il testo selezionato nella finestra RX diventa il nome", "the text selected in the RX window becomes the name", "der im RX-Fenster ausgewählte Text wird zum Namen", "le texte sélectionné dans la fenêtre RX devient le nom", "el texto seleccionado en la ventana RX pasa a ser el nombre", "o texto selecionado na janela RX passa a ser o nome"),
  c('text', "<SELQTH>", "il testo selezionato nella finestra RX diventa il QTH", "the text selected in the RX window becomes the QTH", "der im RX-Fenster ausgewählte Text wird zum QTH", "le texte sélectionné dans la fenêtre RX devient le QTH", "el texto seleccionado en la ventana RX pasa a ser el QTH", "o texto selecionado na janela RX passa a ser o QTH"),
  c('text', "<SELRSTS>", "la parola selezionata diventa il rapporto da mandare", "the selected word becomes the report to send", "das ausgewählte Wort wird zum Rapport, der gesendet wird", "le mot sélectionné devient le report à envoyer", "la palabra seleccionada pasa a ser el reporte que se envía", "a palavra selecionada passa a ser o relatório a enviar"),
  c('text', "<SELRSTR>", "la parola selezionata diventa il rapporto ricevuto", "the selected word becomes the received report", "das ausgewählte Wort wird zum empfangenen Rapport", "le mot sélectionné devient le report reçu", "la palabra seleccionada pasa a ser el reporte recibido", "a palavra selecionada passa a ser o relatório recebido"),
  c('text', "<SELNRS>", "la parola selezionata diventa lo scambio da mandare", "the selected word becomes the exchange to send", "das ausgewählte Wort wird zum Austausch, der gesendet wird", "le mot sélectionné devient l'échange à envoyer", "la palabra seleccionada pasa a ser el intercambio que se envía", "a palavra selecionada passa a ser a troca a enviar"),
  c('text', "<SELNRR>", "la parola selezionata diventa lo scambio ricevuto", "the selected word becomes the received exchange", "das ausgewählte Wort wird zum empfangenen Austausch", "le mot sélectionné devient l'échange reçu", "la palabra seleccionada pasa a ser el intercambio recibido", "a palavra selecionada passa a ser a troca recebida"),
  c('text', "<SELRXTEXT>", "scrive il testo selezionato nella finestra RX", "writes the text selected in the RX window", "schreibt den im RX-Fenster ausgewählten Text", "écrit le texte sélectionné dans la fenêtre RX", "escribe el texto seleccionado en la ventana RX", "escreve o texto selecionado na janela RX"),
  c('text', "<SAVESELTEXT>", "copia negli appunti il testo selezionato nella finestra RX", "copies the text selected in the RX window to the clipboard", "kopiert den im RX-Fenster ausgewählten Text in die Zwischenablage", "copie dans le presse-papiers le texte sélectionné dans la fenêtre RX", "copia al portapapeles el texto seleccionado en la ventana RX", "copia para a área de transferência o texto selecionado na janela RX"),
  c('text', "<INTDE>", "«de» nella lingua del corrispondente (scelta dal prefisso)", "«de» in the other station's language (from the prefix)", "«de» in der Sprache der Gegenstation (aus dem Präfix)", "«de» dans la langue de la station correspondante (d'après le préfixe)", "«de» en el idioma de la estación corresponsal (según el prefijo)", "«de» na língua da estação correspondente (pelo prefixo)"),
  c('text', "<INTQSL>", "«QSL, grazie» nella lingua del corrispondente", "«QSL, thanks» in the other station's language", "«QSL, danke» in der Sprache der Gegenstation", "«QSL, merci» dans la langue de la station correspondante", "«QSL, gracias» en el idioma de la estación corresponsal", "«QSL, obrigado» na língua da estação correspondente"),
  c('text', "<INTAGNNR>", "«ripeti il numero» nella lingua del corrispondente", "«please repeat your number» in the other station's language", "«bitte wiederhole deine Nummer» in der Sprache der Gegenstation", "«merci de répéter ton numéro» dans la langue de la station correspondante", "«repite tu número, por favor» en el idioma de la estación corresponsal", "«repete o teu número, por favor» na língua da estação correspondente"),
  c('text', "<INTAGNCALL>", "«ripeti il nominativo» nella lingua del corrispondente", "«please repeat your call» in the other station's language", "«bitte wiederhole dein Rufzeichen» in der Sprache der Gegenstation", "«merci de répéter ton indicatif» dans la langue de la station correspondante", "«repite tu indicativo, por favor» en el idioma de la estación corresponsal", "«repete o teu indicativo, por favor» na língua da estação correspondente"),
  c('text', "<INTQRZ>", "«chi chiama?» nella lingua del corrispondente", "«who is calling?» in the other station's language", "«wer ruft?» in der Sprache der Gegenstation", "«qui appelle ?» dans la langue de la station correspondante", "«¿quién llama?» en el idioma de la estación corresponsal", "«quem chama?» na língua da estação correspondente"),
  c('text', "<INTGA>", "buongiorno, buon pomeriggio o buonasera nella lingua del corrispondente", "good morning, afternoon or evening in the other station's language", "guten Morgen, guten Tag oder guten Abend in der Sprache der Gegenstation", "bonjour, bon après-midi ou bonsoir dans la langue de la station correspondante", "buenos días, buenas tardes o buenas noches en el idioma de la estación corresponsal", "bom dia, boa tarde ou boa noite na língua da estação correspondente"),
  c('text', "<INTQB4>", "«ci siamo già collegati» nella lingua del corrispondente", "«we worked before» in the other station's language", "«wir haben schon gearbeitet» in der Sprache der Gegenstation", "«nous avons déjà fait un QSO» dans la langue de la station correspondante", "«ya hemos hecho QSO» en el idioma de la estación corresponsal", "«já fizemos QSO» na língua da estação correspondente"),
  c('text', "<INTQRL>", "«la frequenza è libera?» nella lingua del corrispondente", "«is this frequency in use?» in the other station's language", "«ist diese Frequenz belegt?» in der Sprache der Gegenstation", "«cette fréquence est-elle occupée ?» dans la langue de la station correspondante", "«¿está en uso esta frecuencia?» en el idioma de la estación corresponsal", "«esta frequência está em uso?» na língua da estação correspondente"),
  c('text', "<INTCQ>", "«chiamata generale» nella lingua del corrispondente", "«calling CQ» in the other station's language", "«CQ-Ruf» in der Sprache der Gegenstation", "«appel CQ» dans la langue de la station correspondante", "«llamando CQ» en el idioma de la estación corresponsal", "«a chamar CQ» na língua da estação correspondente"),
  c('text', "<INTE>", "«e» nella lingua del corrispondente", "«and» in the other station's language", "«und» in der Sprache der Gegenstation", "«et» dans la langue de la station correspondante", "«y» en el idioma de la estación corresponsal", "«e» na língua da estação correspondente"),
  c('text', "<GM>", "saluto secondo l'ora locale (GM, GA, GE o GN); se hai le macro gm, ga, ge o gn usa quelle", "greeting by local time (GM, GA, GE or GN); if you have the macros gm, ga, ge or gn it uses them", "Gruß nach Ortszeit (GM, GA, GE oder GN); wenn du die Makros gm, ga, ge oder gn hast, werden sie verwendet", "salutation selon l'heure locale (GM, GA, GE ou GN) ; si tu as les macros gm, ga, ge ou gn, elle les utilise", "saludo según la hora local (GM, GA, GE o GN); si tienes las macros gm, ga, ge o gn, las usa", "saudação pela hora local (GM, GA, GE ou GN); se tiveres as macros gm, ga, ge ou gn, usa-as"),
  c('text', "<LANG_xx>", "le frasi INT di questa macro nella lingua xx (IT, EN, DE, FR, ES, PT, NL)", "the INT phrases of this macro in language xx (IT, EN, DE, FR, ES, PT, NL)", "die INT-Sätze dieses Makros in der Sprache xx (IT, EN, DE, FR, ES, PT, NL)", "les phrases INT de cette macro dans la langue xx (IT, EN, DE, FR, ES, PT, NL)", "las frases INT de esta macro en el idioma xx (IT, EN, DE, FR, ES, PT, NL)", "as frases INT desta macro na língua xx (IT, EN, DE, FR, ES, PT, NL)"),
  c('text', "<FORCELANGUAGE:xx>", "le frasi INT restano nella lingua xx finché non la cambi (vuoto: di nuovo dal nominativo)", "the INT phrases stay in language xx until you change it (empty: from the callsign again)", "die INT-Sätze bleiben in der Sprache xx, bis du sie änderst (leer: wieder nach dem Rufzeichen)", "les phrases INT restent dans la langue xx jusqu'à ce que tu la changes (vide : de nouveau d'après l'indicatif)", "las frases INT se quedan en el idioma xx hasta que lo cambies (vacío: otra vez según el indicativo)", "as frases INT ficam na língua xx até a mudares (vazio: outra vez pelo indicativo)"),
  c('text', "<TXTIME>", "ora UTC dell'inizio dell'ultima trasmissione", "UTC time the last transmission started", "UTC-Zeit, zu der die letzte Aussendung begann", "heure UTC du début de la dernière émission", "hora UTC en que empezó la última transmisión", "hora UTC a que começou a última transmissão"),
  c('text', "<LOCALDATE>", "data locale", "local date", "lokales Datum", "date locale", "fecha local", "data local"),
  c('text', "<TIME:formato>", "ora UTC con un formato: %H %M %S ore minuti secondi, %d %m %Y %y giorno mese anno, %j giorno dell'anno, %b mese (vale anche per DATE, LOCALTIME, CTIME)", "UTC time with a format: %H %M %S hours minutes seconds, %d %m %Y %y day month year, %j day of year, %b month (also for DATE, LOCALTIME, CTIME)", "UTC-Zeit mit Format: %H %M %S Stunden Minuten Sekunden, %d %m %Y %y Tag Monat Jahr, %j Tag des Jahres, %b Monat (auch für DATE, LOCALTIME, CTIME)", "heure UTC avec un format : %H %M %S heures minutes secondes, %d %m %Y %y jour mois année, %j jour de l'année, %b mois (aussi pour DATE, LOCALTIME, CTIME)", "hora UTC con formato: %H %M %S horas minutos segundos, %d %m %Y %y día mes año, %j día del año, %b mes (también para DATE, LOCALTIME, CTIME)", "hora UTC com um formato: %H %M %S horas minutos segundos, %d %m %Y %y dia mês ano, %j dia do ano, %b mês (também para DATE, LOCALTIME, CTIME)"),
  c('program', "<STARTTIME:hhmm>", "fissa l'ora di inizio del QSO (con % scrive l'ora di inizio con quel formato)", "sets the QSO start time (with % it writes the start time in that format)", "setzt die Startzeit des QSO (mit % schreibt es die Startzeit in diesem Format)", "règle l'heure de début du QSO (avec % elle écrit l'heure de début dans ce format)", "fija la hora de inicio del QSO (con % escribe la hora de inicio en ese formato)", "define a hora de início do QSO (com % escreve a hora de início nesse formato)"),
  c('program', "<TIMER:s,macro>", "dopo s secondi esegue la macro (senza macro suona un avviso); <TIMER:0> lo ferma", "after s seconds runs the macro (without a macro it sounds an alert); <TIMER:0> stops it", "startet nach s Sekunden das Makro (ohne Makro ertönt ein Alarm); <TIMER:0> stoppt es", "après s secondes exécute la macro (sans macro, elle émet une alerte) ; <TIMER:0> l'arrête", "tras s segundos ejecuta la macro (sin macro suena una alerta); <TIMER:0> lo detiene", "passados s segundos executa a macro (sem macro toca um alerta); <TIMER:0> pára-o"),
  c('program', "<CLEARNRR>", "svuota lo scambio ricevuto", "clears the received exchange", "löscht den empfangenen Austausch", "efface l'échange reçu", "borra el intercambio recibido", "limpa a troca recebida"),
  c('program', "<INCNRR>", "lo scambio ricevuto più uno (stessi zeri davanti)", "the received exchange plus one (same leading zeros)", "der empfangene Austausch plus eins (gleiche führende Nullen)", "l'échange reçu plus un (mêmes zéros initiaux)", "el intercambio recibido más uno (mismos ceros a la izquierda)", "a troca recebida mais um (mesmos zeros à esquerda)"),
  c('program', "<CORRECTCALLTX>", "ritrasmette il nominativo, dopo che l'hai corretto", "sends the callsign again after you corrected it", "sendet das Rufzeichen erneut, nachdem du es korrigiert hast", "renvoie l'indicatif après que tu l'as corrigé", "vuelve a enviar el indicativo después de que lo corrijas", "volta a enviar o indicativo depois de o corrigires"),
  c('program', "<SAVE>", "salva il QSO nel log (come <SAVEQSO>)", "saves the QSO in the log (like <SAVEQSO>)", "speichert das QSO im Log (wie <SAVEQSO>)", "enregistre le QSO dans le log (comme <SAVEQSO>)", "guarda el QSO en el log (como <SAVEQSO>)", "guarda o QSO no log (como <SAVEQSO>)"),
  c('program', "<SAVESHAUTO>", "salva il QSO e svuota i campi", "saves the QSO and clears the fields", "speichert das QSO und leert die Felder", "enregistre le QSO et efface les champs", "guarda el QSO y borra los campos", "guarda o QSO e limpa os campos"),
  c('program', "<READLOG>", "rilegge il log e i suoi segni", "reloads the log and its marks", "lädt das Log und seine Markierungen neu", "recharge le log et ses marques", "recarga el log y sus marcas", "recarrega o log e as suas marcas"),
  c('program', "<WKD>", "dice a schermo se e quando hai già lavorato il nominativo", "shows whether and when you already worked the callsign", "zeigt an, ob und wann du das Rufzeichen schon gearbeitet hast", "indique si et quand tu as déjà contacté l'indicatif", "muestra si ya trabajaste el indicativo y cuándo", "mostra se já trabalhaste o indicativo e quando"),
  c('program', "<WKDIGNOREBEFORE:data>", "come <IGNOREBEFORE:data>", "like <IGNOREBEFORE:date>", "wie <IGNOREBEFORE:date>", "comme <IGNOREBEFORE:date>", "como <IGNOREBEFORE:date>", "como <IGNOREBEFORE:date>"),
  c('program', "<ERASE>", "toglie il limite di <IGNOREBEFORE>: i segni del log contano di nuovo tutti i QSO", "removes the <IGNOREBEFORE> limit: log marks count all QSOs again", "hebt die Grenze von <IGNOREBEFORE> auf: die Log-Markierungen zählen wieder alle QSOs", "supprime la limite de <IGNOREBEFORE> : les marques du log comptent de nouveau tous les QSO", "quita el límite de <IGNOREBEFORE>: las marcas del log vuelven a contar todos los QSO", "remove o limite de <IGNOREBEFORE>: as marcas do log voltam a contar todos os QSO"),
  c('program', "<PRINTQSL>", "stampa l'etichetta QSL del QSO in corso", "prints the QSL label of the current QSO", "druckt das QSL-Etikett des aktuellen QSO", "imprime l'étiquette QSL du QSO en cours", "imprime la etiqueta QSL del QSO actual", "imprime a etiqueta QSL do QSO atual"),
  c('program', "<SENDEQSL>", "conferme in rete: Aurora manda ogni QSO salvato ai servizi scelti nelle Impostazioni (eQSL, LoTW, Club Log...), il comando lo ricorda", "online confirmations: Aurora sends every saved QSO to the services chosen in Settings (eQSL, LoTW, Club Log...), the command reminds you", "Online-Bestätigungen: Aurora sendet jedes gespeicherte QSO an die in den Einstellungen gewählten Dienste (eQSL, LoTW, Club Log...), der Befehl erinnert dich daran", "confirmations en ligne : Aurora envoie chaque QSO enregistré aux services choisis dans les Paramètres (eQSL, LoTW, Club Log...), la commande te le rappelle", "confirmaciones en línea: Aurora envía cada QSO guardado a los servicios elegidos en Ajustes (eQSL, LoTW, Club Log...), el comando te lo recuerda", "confirmações online: o Aurora envia cada QSO guardado para os serviços escolhidos nas Definições (eQSL, LoTW, Club Log...), o comando lembra-te disso"),
  c('program', "<SENDSPOT:commento>", "manda al DX cluster uno spot del corrispondente sulla frequenza del segnale", "sends a DX cluster spot of the other station on the signal frequency", "sendet einen DX-Cluster-Spot der Gegenstation auf der Signalfrequenz", "envoie un spot DX cluster de la station correspondante sur la fréquence du signal", "envía un spot de DX cluster de la estación corresponsal en la frecuencia de la señal", "envia um spot de DX cluster da estação correspondente na frequência do sinal"),
  c('program', "<QSOCMDDDE:comando>", "Aurora non usa il DDE: i QSO salvati arrivano già ai programmi di log collegati", "Aurora does not use DDE: saved QSOs already reach the connected logging programs", "Aurora nutzt kein DDE: gespeicherte QSOs erreichen die verbundenen Logprogramme bereits", "Aurora n'utilise pas DDE : les QSO enregistrés arrivent déjà aux programmes de log connectés", "Aurora no usa DDE: los QSO guardados ya llegan a los programas de log conectados", "O Aurora não usa DDE: os QSO guardados já chegam aos programas de log ligados"),
  c('mode', "<LOCKMODE>", "blocca il modo: <MODE:...> non lo cambia più (<UNLOCKMODE> lo libera)", "locks the mode: <MODE:...> no longer changes it (<UNLOCKMODE> frees it)", "sperrt den Modus: <MODE:...> ändert ihn nicht mehr (<UNLOCKMODE> gibt ihn frei)", "verrouille le mode : <MODE:...> ne le change plus (<UNLOCKMODE> le libère)", "bloquea el modo: <MODE:...> ya no lo cambia (<UNLOCKMODE> lo libera)", "bloqueia o modo: <MODE:...> já não o muda (<UNLOCKMODE> liberta-o)"),
  c('mode', "<LOCKRX>", "blocca la frequenza di ricezione: clic e macro non la spostano (<UNLOCKRX>, <LOCKRXTOGGLE>)", "locks the receive frequency: clicks and macros do not move it (<UNLOCKRX>, <LOCKRXTOGGLE>)", "sperrt die Empfangsfrequenz: Klicks und Makros verschieben sie nicht (<UNLOCKRX>, <LOCKRXTOGGLE>)", "verrouille la fréquence de réception : les clics et les macros ne la déplacent pas (<UNLOCKRX>, <LOCKRXTOGGLE>)", "bloquea la frecuencia de recepción: los clics y las macros no la mueven (<UNLOCKRX>, <LOCKRXTOGGLE>)", "bloqueia a frequência de receção: os cliques e as macros não a movem (<UNLOCKRX>, <LOCKRXTOGGLE>)"),
  c('mode', "<MODEADD:modo>", "come <MODE:modo>", "like <MODE:mode>", "wie <MODE:mode>", "comme <MODE:mode>", "como <MODE:mode>", "como <MODE:mode>"),
  c('mode', "<WORDMODE:ON>", "mentre scrivi la trasmissione parte a parole intere (OFF: lettera per lettera)", "while you type, the transmission goes out in whole words (OFF: letter by letter)", "während du tippst, wird die Aussendung in ganzen Wörtern gesendet (OFF: Buchstabe für Buchstabe)", "pendant que tu tapes, l'émission part par mots entiers (OFF : lettre par lettre)", "mientras escribes, la transmisión sale en palabras completas (OFF: letra a letra)", "enquanto escreves, a transmissão sai em palavras inteiras (OFF: letra a letra)"),
  c('mode', "<CAPITALSONLY:ON>", "solo maiuscole nella finestra di trasmissione (OFF le toglie)", "capitals only in the transmit window (OFF turns it off)", "nur Großbuchstaben im Sendefenster (OFF schaltet es aus)", "majuscules uniquement dans la fenêtre d'émission (OFF le désactive)", "solo mayúsculas en la ventana de transmisión (OFF lo desactiva)", "só maiúsculas na janela de transmissão (OFF desativa)"),
  c('mode', "<CWID>", "in RTTY e PSK aggiunge DE e il tuo nominativo in fondo (<CWID:testo> un altro testo)", "in RTTY and PSK adds DE and your callsign at the end (<CWID:text> another text)", "bei RTTY und PSK hängt es am Ende DE und dein Rufzeichen an (<CWID:text> anderer Text)", "en RTTY et PSK ajoute DE et ton indicatif à la fin (<CWID:text> un autre texte)", "en RTTY y PSK añade DE y tu indicativo al final (<CWID:text> otro texto)", "em RTTY e PSK acrescenta DE e o teu indicativo no fim (<CWID:text> outro texto)"),
  c('mode', "<REVERSEPADDLE>", "scambia punto e linea del paddle del WinKeyer", "swaps dot and dash of the WinKeyer paddle", "vertauscht Punkt und Strich der WinKeyer-Paddle", "inverse le point et le trait de la manipulation WinKeyer", "intercambia punto y raya de la pala del WinKeyer", "troca ponto e traço da palheta do WinKeyer"),
  c('mode', "<SNAPNOW>", "porta subito la ricezione sul segnale più vicino", "moves the receive onto the nearest signal now", "verschiebt den Empfang jetzt auf das nächstgelegene Signal", "déplace maintenant la réception sur le signal le plus proche", "mueve ahora la recepción a la señal más cercana", "move agora a receção para o sinal mais próximo"),
  c('mode', "<INV>", "RTTY: inverte mark e space (<INVON>, <INVOFF>)", "RTTY: swaps mark and space (<INVON>, <INVOFF>)", "RTTY: vertauscht Mark und Space (<INVON>, <INVOFF>)", "RTTY : inverse mark et space (<INVON>, <INVOFF>)", "RTTY: intercambia mark y space (<INVON>, <INVOFF>)", "RTTY: troca mark e space (<INVON>, <INVOFF>)"),
  c('mode', "<FILTER:PASS>", "filtro di ricezione: PASS passabanda stretto, NONE spento (anche i notch); NOTCH e NOISE accendono il passabanda", "receive filter: PASS narrow passband, NONE off (notches too); NOTCH and NOISE turn the passband on", "Empfangsfilter: PASS schmaler Durchlassbereich, NONE aus (auch Notches); NOTCH und NOISE schalten den Durchlassbereich ein", "filtre de réception : PASS bande passante étroite, NONE désactivé (notchs aussi) ; NOTCH et NOISE activent la bande passante", "filtro de recepción: PASS banda pasante estrecha, NONE desactivado (también los notch); NOTCH y NOISE activan la banda pasante", "filtro de receção: PASS banda passante estreita, NONE desligado (notches também); NOTCH e NOISE ligam a banda passante"),
  c('mode', "<BROADCASTMODE:ON>", "ON: si resta in trasmissione anche se la macro ha <RX>, finché non premi RX; OFF torna normale", "ON: you stay in transmit even if the macro has <RX>, until you press RX; OFF goes back to normal", "ON: du bleibst im Senden, auch wenn das Makro <RX> enthält, bis du RX drückst; OFF kehrt zum Normalzustand zurück", "ON : tu restes en émission même si la macro contient <RX>, jusqu'à ce que tu appuies sur RX ; OFF revient à la normale", "ON: sigues en transmisión aunque la macro tenga <RX>, hasta que pulses RX; OFF vuelve a lo normal", "ON: ficas em transmissão mesmo que a macro tenha <RX>, até carregares em RX; OFF volta ao normal"),
  c('mode', "<TOGGLECHANNEL>", "scambia la ricezione principale con la prima finestra RX in più", "swaps the main receive with the first extra RX window", "tauscht den Hauptempfang mit dem ersten zusätzlichen RX-Fenster", "échange la réception principale avec la première fenêtre RX supplémentaire", "intercambia la recepción principal con la primera ventana RX adicional", "troca a receção principal com a primeira janela RX adicional"),
  c('program', "<ESCAPE>", "ferma subito la trasmissione", "stops the transmission at once", "stoppt die Aussendung sofort", "arrête l'émission immédiatement", "detiene la transmisión de inmediato", "pára a transmissão de imediato"),
  c('program', "<PTT:ON>", "apre la trasmissione (OFF la chiude)", "opens the transmission (OFF closes it)", "öffnet die Aussendung (OFF schließt sie)", "ouvre l'émission (OFF la ferme)", "abre la transmisión (OFF la cierra)", "abre a transmissão (OFF fecha-a)"),
  c('mode', "<SET BW:Hz>", "un parametro del modo: BW (filtro della CW o passabanda), SHIFT, BAUD, WPM, SQUELCH, SUBMODE (BPSK31, PSK63, RTTY, CW); TONES e i sottomodi che Aurora non ha si saltano", "a mode parameter: BW (CW filter or passband), SHIFT, BAUD, WPM, SQUELCH, SUBMODE (BPSK31, PSK63, RTTY, CW); TONES and submodes Aurora lacks are skipped", "ein Modusparameter: BW (CW-Filter oder Durchlassbereich), SHIFT, BAUD, WPM, SQUELCH, SUBMODE (BPSK31, PSK63, RTTY, CW); TONES und Submodi, die Aurora nicht hat, werden übersprungen", "un paramètre du mode : BW (filtre CW ou bande passante), SHIFT, BAUD, WPM, SQUELCH, SUBMODE (BPSK31, PSK63, RTTY, CW) ; TONES et les sous-modes qu'Aurora n'a pas sont ignorés", "un parámetro del modo: BW (filtro de CW o banda pasante), SHIFT, BAUD, WPM, SQUELCH, SUBMODE (BPSK31, PSK63, RTTY, CW); TONES y los submodos que Aurora no tiene se omiten", "um parâmetro do modo: BW (filtro de CW ou banda passante), SHIFT, BAUD, WPM, SQUELCH, SUBMODE (BPSK31, PSK63, RTTY, CW); TONES e os submodos que o Aurora não tem são ignorados"),
  c('mode', "<GET SN>", "scrive un valore del segnale: SN (S/N), OFFSET (frequenza audio), RATE (baud o WPM), MODE, IMD", "writes a signal value: SN (S/N), OFFSET (audio frequency), RATE (baud or WPM), MODE, IMD", "schreibt einen Signalwert: SN (S/N), OFFSET (Audiofrequenz), RATE (Baud oder WPM), MODE, IMD", "écrit une valeur du signal : SN (S/N), OFFSET (fréquence audio), RATE (baud ou WPM), MODE, IMD", "escribe un valor de la señal: SN (S/N), OFFSET (frecuencia de audio), RATE (baudios o WPM), MODE, IMD", "escreve um valor do sinal: SN (S/N), OFFSET (frequência de áudio), RATE (baud ou WPM), MODE, IMD"),
  c('frequency', "<SPZOOM:n>", "cascata: 1 larghezza normale, 2 o più cascata larga", "waterfall: 1 normal width, 2 or more wide waterfall", "Wasserfall: 1 normale Breite, 2 oder mehr breiter Wasserfall", "cascade : 1 largeur normale, 2 ou plus cascade large", "cascada: 1 anchura normal, 2 o más cascada ancha", "cascata: 1 largura normal, 2 ou mais cascata larga"),
  c('frequency', "<SPSHIFT:Hz>", "porta la ricezione su questa frequenza audio della cascata", "moves the receive to this audio frequency of the waterfall", "verschiebt den Empfang auf diese Audiofrequenz des Wasserfalls", "déplace la réception sur cette fréquence audio de la cascade", "mueve la recepción a esta frecuencia de audio de la cascada", "move a receção para esta frequência de áudio da cascata"),
  c('frequency', "<AMPLITUDE:dB>", "livello di uscita: attenuazione in dB (0 = massimo)", "output level: attenuation in dB (0 = maximum)", "Ausgangspegel: Dämpfung in dB (0 = Maximum)", "niveau de sortie : atténuation en dB (0 = maximum)", "nivel de salida: atenuación en dB (0 = máximo)", "nível de saída: atenuação em dB (0 = máximo)"),
  c('frequency', "<ZFQ>", "sposta la radio perché il segnale in ricezione cada a 1500 Hz", "moves the radio so the received signal falls at 1500 Hz", "verschiebt das Funkgerät so, dass das empfangene Signal bei 1500 Hz liegt", "déplace la radio pour que le signal reçu tombe à 1500 Hz", "mueve la radio para que la señal recibida caiga en 1500 Hz", "move o rádio para que o sinal recebido caia nos 1500 Hz"),
  c('frequency', "<FQB:kHz>", "frequenza di trasmissione (VFO B, split) in kHz", "transmit frequency (VFO B, split) in kHz", "Sendefrequenz (VFO B, Split) in kHz", "fréquence d'émission (VFO B, split) en kHz", "frecuencia de transmisión (VFO B, split) en kHz", "frequência de transmissão (VFO B, split) em kHz"),
  c('text', "<TEST>", "scrive TEST se c'è un contest attivo, se no niente", "writes TEST if a contest is active, otherwise nothing", "schreibt TEST, wenn ein Contest aktiv ist, sonst nichts", "écrit TEST si un concours est actif, sinon rien", "escribe TEST si hay un concurso activo, si no, nada", "escreve TEST se houver um concurso ativo, caso contrário nada"),
  c('frequency', "<CATCMDHEX:byte>", "manda alla radio un comando CAT scritto in esadecimale (radio Icom e altre a byte)", "sends the radio a CAT command written in hex (Icom and other binary radios)", "sendet dem Funkgerät einen in Hex geschriebenen CAT-Befehl (Icom und andere Binär-Funkgeräte)", "envoie à la radio une commande CAT écrite en hexadécimal (Icom et autres radios binaires)", "envía a la radio un comando CAT escrito en hexadecimal (Icom y otras radios binarias)", "envia ao rádio um comando CAT escrito em hexadecimal (Icom e outros rádios binários)"),
  c('frequency', "<CATCMDX:comando>", "come <CATCMD>: accetta anche i byte scritti \\0xNN", "like <CATCMD>: also accepts bytes written \\0xNN", "wie <CATCMD>: akzeptiert auch als \\0xNN geschriebene Bytes", "comme <CATCMD> : accepte aussi les octets écrits \\0xNN", "como <CATCMD>: acepta también bytes escritos \\0xNN", "como <CATCMD>: aceita também bytes escritos \\0xNN"),
  c('frequency', "<CATWAIT:ms>", "aspetta questi millisecondi prima del comando CAT seguente", "waits these milliseconds before the next CAT command", "wartet diese Millisekunden vor dem nächsten CAT-Befehl", "attend ces millisecondes avant la commande CAT suivante", "espera estos milisegundos antes del siguiente comando CAT", "espera estes milissegundos antes do comando CAT seguinte"),
  c('frequency', "<CATTRANSVERTER:MHz>", "spostamento del transverter: lo si regola nelle Impostazioni della radio, il comando apre l'avviso", "transverter offset: set it in the radio settings, the command shows a notice", "Transverter-Offset: stelle ihn in den Funkgeräte-Einstellungen ein, der Befehl zeigt einen Hinweis", "décalage du transverter : règle-le dans les paramètres de la radio, la commande affiche un avis", "desplazamiento del transverter: configúralo en los ajustes de la radio, el comando muestra un aviso", "desvio do transverter: define-o nas definições do rádio, o comando mostra um aviso"),
  c('frequency', "<CATVFOSYNCONCE>", "sposta una volta la radio perché il segnale cada sul centro (1500 Hz o quello di <CATVFOSYNCSHIFT:Hz>)", "moves the radio once so the signal falls on the centre (1500 Hz or that of <CATVFOSYNCSHIFT:Hz>)", "verschiebt das Funkgerät einmal so, dass das Signal auf die Mitte fällt (1500 Hz oder die von <CATVFOSYNCSHIFT:Hz>)", "déplace une fois la radio pour que le signal tombe au centre (1500 Hz ou celui de <CATVFOSYNCSHIFT:Hz>)", "mueve la radio una vez para que la señal caiga en el centro (1500 Hz o el de <CATVFOSYNCSHIFT:Hz>)", "move o rádio uma vez para que o sinal caia no centro (1500 Hz ou o de <CATVFOSYNCSHIFT:Hz>)"),
  c('frequency', "<CATENABLEVFOSYNC>", "sincronia della radio: a ogni macro il segnale torna al centro (<CATDISABLEVFOSYNC> la spegne)", "radio sync: at every macro the signal goes back to the centre (<CATDISABLEVFOSYNC> turns it off)", "Funkgeräte-Sync: bei jedem Makro kehrt das Signal in die Mitte zurück (<CATDISABLEVFOSYNC> schaltet es aus)", "synchro radio : à chaque macro le signal revient au centre (<CATDISABLEVFOSYNC> la désactive)", "sincronización de la radio: en cada macro la señal vuelve al centro (<CATDISABLEVFOSYNC> la desactiva)", "sincronização do rádio: a cada macro o sinal volta ao centro (<CATDISABLEVFOSYNC> desliga-a)"),
  c('frequency', "<TOGGLECATPORT>", "collega o scollega la radio (CAT); anche <TOGGLEPTTPORT>, perché in Aurora il PTT passa dal CAT", "connects or disconnects the radio (CAT); <TOGGLEPTTPORT> too, since in Aurora PTT goes through CAT", "verbindet oder trennt das Funkgerät (CAT); auch <TOGGLEPTTPORT>, da in Aurora PTT über CAT läuft", "connecte ou déconnecte la radio (CAT) ; aussi <TOGGLEPTTPORT>, car dans Aurora le PTT passe par le CAT", "conecta o desconecta la radio (CAT); también <TOGGLEPTTPORT>, porque en Aurora el PTT pasa por CAT", "liga ou desliga o rádio (CAT); também <TOGGLEPTTPORT>, porque no Aurora o PTT passa pelo CAT"),
  c('frequency', "<TOGGLESOUNDDEV>", "riapre l'ascolto della scheda audio (spegne e riaccende la ricezione)", "reopens listening on the sound card (stops and restarts receive)", "öffnet das Hören auf der Soundkarte neu (stoppt den Empfang und startet ihn neu)", "rouvre l'écoute sur la carte son (arrête et relance la réception)", "vuelve a abrir la escucha en la tarjeta de sonido (detiene y reinicia la recepción)", "reabre a escuta na placa de som (pára e reinicia a receção)"),
  c('frequency', "<INPUTVOLUME:%>", "volume dell'ingresso audio in percento (senza numero apre le impostazioni audio)", "audio input volume in percent (without a number opens the audio settings)", "Lautstärke des Audioeingangs in Prozent (ohne Zahl öffnet die Audio-Einstellungen)", "volume d'entrée audio en pourcentage (sans nombre, ouvre les paramètres audio)", "volumen de entrada de audio en porcentaje (sin número abre los ajustes de audio)", "volume de entrada de áudio em percentagem (sem número abre as definições de áudio)"),
  c('frequency', "<OUTPUTVOLUME:dB>", "attenuazione dell'uscita audio in dB (senza numero apre le impostazioni audio)", "audio output attenuation in dB (without a number opens the audio settings)", "Dämpfung des Audioausgangs in dB (ohne Zahl öffnet die Audio-Einstellungen)", "atténuation de la sortie audio en dB (sans nombre, ouvre les paramètres audio)", "atenuación de la salida de audio en dB (sin número abre los ajustes de audio)", "atenuação da saída de áudio em dB (sem número abre as definições de áudio)"),
  c('cw', "<WKTEXT:testo>", "manda il testo in CW con il WinKeyer", "sends the text in CW with the WinKeyer", "sendet den Text in CW mit dem WinKeyer", "envoie le texte en CW avec le WinKeyer", "envía el texto en CW con el WinKeyer", "envia o texto em CW com o WinKeyer"),
  c('cw', "<WKHEX:byte>", "manda al WinKeyer byte di comando in esadecimale", "sends command bytes in hex to the WinKeyer", "sendet Befehlsbytes in Hex an den WinKeyer", "envoie des octets de commande en hexadécimal au WinKeyer", "envía bytes de comando en hexadecimal al WinKeyer", "envia bytes de comando em hexadecimal ao WinKeyer"),
  c('cw', "<WKPOT>", "la velocità la decide il potenziometro del WinKeyer (<WKNOPOT>: la decide Aurora)", "the WinKeyer knob sets the speed (<WKNOPOT>: Aurora sets it)", "der WinKeyer-Drehknopf stellt die Geschwindigkeit ein (<WKNOPOT>: Aurora stellt sie ein)", "le bouton du WinKeyer règle la vitesse (<WKNOPOT> : Aurora la règle)", "el mando del WinKeyer fija la velocidad (<WKNOPOT>: la fija Aurora)", "o botão do WinKeyer define a velocidade (<WKNOPOT>: o Aurora define-a)"),
  c('rotor', "<ROTORCMD:comando>", "comando del rotore alla GS-232: M o W con i gradi gira, S o A ferma; si fa con il rotore di Aurora", "GS-232 style rotator command: M or W with degrees turns, S or A stops; done with Aurora's rotator", "Rotor-Befehl im Stil von GS-232: M oder W mit Grad dreht, S oder A stoppt; erledigt mit dem Rotor von Aurora", "commande de rotor de type GS-232 : M ou W avec des degrés tourne, S ou A arrête ; exécutée avec le rotor d'Aurora", "comando de rotor estilo GS-232: M o W con grados gira, S o A detiene; se hace con el rotor de Aurora", "comando de rotor ao estilo GS-232: M ou W com graus roda, S ou A pára; feito com o rotor do Aurora"),
  c('rotor', "<ROTORCMDHEX:byte>", "come <ROTORCMD> con il comando scritto in esadecimale", "like <ROTORCMD> with the command written in hex", "wie <ROTORCMD> mit dem Befehl in Hex geschrieben", "comme <ROTORCMD> avec la commande écrite en hexadécimal", "como <ROTORCMD> con el comando escrito en hexadecimal", "como <ROTORCMD> com o comando escrito em hexadecimal"),
  c('macros', "<F1>", "esegue un'altra macro (tasto o nome, come <Hello>) dentro questa, fino a 10 livelli", "runs another macro (key or name, like <Hello>) inside this one, up to 10 levels", "führt ein anderes Makro (Taste oder Name, wie <Hello>) innerhalb dieses Makros aus, bis zu 10 Ebenen", "exécute une autre macro (touche ou nom, comme <Hello>) à l'intérieur de celle-ci, jusqu'à 10 niveaux", "ejecuta otra macro (tecla o nombre, como <Hello>) dentro de esta, hasta 10 niveles", "executa outra macro (tecla ou nome, como <Hello>) dentro desta, até 10 níveis"),
  c('macros', "<CALL?>", "chiede il valore se è vuoto (anche <RSTS?>, <RSTR?>, <NRS?>, <NRR?>)", "asks for the value if it is empty (also <RSTS?>, <RSTR?>, <NRS?>, <NRR?>)", "fragt nach dem Wert, wenn er leer ist (auch <RSTS?>, <RSTR?>, <NRS?>, <NRR?>)", "demande la valeur si elle est vide (aussi <RSTS?>, <RSTR?>, <NRS?>, <NRR?>)", "pide el valor si está vacío (también <RSTS?>, <RSTR?>, <NRS?>, <NRR?>)", "pede o valor se estiver vazio (também <RSTS?>, <RSTR?>, <NRS?>, <NRR?>)"),
  c('macros', "<F1?>", "apre quella macro per modificarla", "opens that macro to edit it", "öffnet dieses Makro zum Bearbeiten", "ouvre cette macro pour la modifier", "abre esa macro para editarla", "abre essa macro para a editar"),
];

/** Le categorie della «Guida alle macro» (pagina dei modi testuali). */
export type HelpCategory = 'data' | 'prev' | 'select' | 'phrases' | 'time' | 'format' | 'freq' | 'txrx' | 'windows' | 'qso' | 'mode' | 'autocq' | 'rotor' | 'cat' | 'keyer' | 'audio' | 'files' | 'nested' | 'cw' | 'rtty' | 'sstv' | 'hell' | 'packet';

export const HELP_CATEGORIES: readonly HelpCategory[] = ['data', 'prev', 'select', 'phrases', 'time', 'format', 'freq', 'txrx', 'windows', 'qso', 'mode', 'autocq', 'rotor', 'cat', 'keyer', 'audio', 'files', 'nested', 'cw', 'rtty', 'sstv', 'hell', 'packet'];

const CATEGORY: Record<string, HelpCategory> = {
  '<MYCALL>': 'data', '<MYMAINCALL>': 'data', '<MYNAME>': 'data', '<MYQTH>': 'data', '<MYWWLOC>': 'data', '<CALL>': 'data', '<NAME>': 'data',
  '<QTH>': 'data', '<WWLOC>': 'data', '<RSTS>': 'data', '<RSTR>': 'data', '<NOTES>': 'data', '<ANYQSOBEFOREDATE>': 'data',
  '<ANYQSOBEFOREBAND>': 'data', '<ANYQSOBEFOREMODE>': 'data', '<PREVNRS>': 'data', '<PREVCALL>': 'data', '<PREVRSTS>': 'data', '<QSOBEFORETIME>': 'data', '<QSOBEFOREDATE>': 'data', '<IMD>': 'data', '<VER>': 'data',
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
  '<ANYQSOBEFOREBAND>': 'LAST QSO ON <ANYQSOBEFOREBAND>', '<ANYQSOBEFOREMODE>': 'LAST QSO IN <ANYQSOBEFOREMODE>', '<PREVNRS>': 'NR <PREVNRS> <PREVNRS>', '<PREVCALL>': '<PREVCALL> TU', '<PREVRSTS>': '<PREVCALL> <PREVRSTS> <PREVNRS>', '<QSOBEFORETIME>': 'QSO B4 <QSOBEFORETIME>', '<QSOBEFOREDATE>': 'QSO B4 <QSOBEFOREDATE>', '<IMD>': 'IMD <IMD>', '<VER>': 'SW <VER>',
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

/** Categoria, esempio e altri nomi dei comandi aggiunti famiglia per famiglia. */
CATEGORY["<ACALL>"] = 'prev';
EXAMPLE["<ACALL>"] = "<ACALL> de <MYCALL>";
CATEGORY["<CALLPOST:testo>"] = 'prev';
EXAMPLE["<CALLPOST:testo>"] = "<CALLPOST: TU>";
CATEGORY["<POST:sì,no>"] = 'prev';
EXAMPLE["<POST:sì,no>"] = "<POST:QRZ?,CQ>";
CATEGORY["<OM>"] = 'prev';
EXAMPLE["<OM>"] = "TU <OM> <NAME>";
CATEGORY["<DIST>"] = 'prev';
EXAMPLE["<DIST>"] = "QRB <DIST> km";
CATEGORY["<MILES>"] = 'prev';
EXAMPLE["<MILES>"] = "QRB <MILES> mi";
CATEGORY["<AZ>"] = 'prev';
EXAMPLE["<AZ>"] = "BEAM <AZ>";
CATEGORY["<PREVNRR>"] = 'prev';
EXAMPLE["<PREVNRR>"] = "PREV NR <PREVNRR>";
CATEGORY["<PREVCCALL>"] = 'prev';
EXAMPLE["<PREVCCALL>"] = "<PREVCCALL> TU";
CATEGORY["<PREVNAME>"] = 'prev';
EXAMPLE["<PREVNAME>"] = "TU <PREVNAME>";
CATEGORY["<PREVQTH>"] = 'prev';
EXAMPLE["<PREVQTH>"] = "<PREVQTH>";
CATEGORY["<PREVRSTR>"] = 'prev';
EXAMPLE["<PREVRSTR>"] = "UR RST WAS <PREVRSTR>";
CATEGORY["<PREVCTIME>"] = 'prev';
EXAMPLE["<PREVCTIME>"] = "QSO AT <PREVCTIME>";
CATEGORY["<PREVSUF2>"] = 'prev';
EXAMPLE["<PREVSUF2>"] = "<PREVSUF2> TU";
CATEGORY["<LASTCALL>"] = 'prev';
EXAMPLE["<LASTCALL>"] = "<LASTCALL><CALL> AGN";
CATEGORY["<NRRPOST:testo>"] = 'prev';
EXAMPLE["<NRRPOST:testo>"] = "<NRRPOST: TU>";
CATEGORY["<QSOBEFORE>"] = 'prev';
EXAMPLE["<QSOBEFORE>"] = "QSO B4 <QSOBEFORE>";
CATEGORY["<QSOBEFORETIME1>"] = 'prev';
EXAMPLE["<QSOBEFORETIME1>"] = "B4 <QSOBEFORETIME1>";
ALIASES["<QSOBEFORETIME1>"] = ["<TIME1>"];
CATEGORY["<QSOBEFOREBAND>"] = 'prev';
EXAMPLE["<QSOBEFOREBAND>"] = "B4 <QSOBEFOREBAND>";
CATEGORY["<QSOBEFOREMODE>"] = 'prev';
EXAMPLE["<QSOBEFOREMODE>"] = "B4 <QSOBEFOREMODE>";
CATEGORY["<ONQSOBEFORE:macro>"] = 'prev';
EXAMPLE["<ONQSOBEFORE:macro>"] = "<ONQSOBEFORE:F9>";
CATEGORY["<SELCALL>"] = 'select';
EXAMPLE["<SELCALL>"] = "<SELCALL><CALL> de <MYCALL>";
CATEGORY["<SELNAME>"] = 'select';
EXAMPLE["<SELNAME>"] = "<SELNAME>TU <NAME>";
CATEGORY["<SELQTH>"] = 'select';
EXAMPLE["<SELQTH>"] = "<SELQTH>";
CATEGORY["<SELRSTS>"] = 'select';
EXAMPLE["<SELRSTS>"] = "<SELRSTS>";
CATEGORY["<SELRSTR>"] = 'select';
EXAMPLE["<SELRSTR>"] = "<SELRSTR>";
CATEGORY["<SELNRS>"] = 'select';
EXAMPLE["<SELNRS>"] = "<SELNRS>";
CATEGORY["<SELNRR>"] = 'select';
EXAMPLE["<SELNRR>"] = "<SELNRR>";
CATEGORY["<SELRXTEXT>"] = 'select';
EXAMPLE["<SELRXTEXT>"] = "I COPY: <SELRXTEXT>";
CATEGORY["<SAVESELTEXT>"] = 'select';
EXAMPLE["<SAVESELTEXT>"] = "<SAVESELTEXT>";
CATEGORY["<INTDE>"] = 'phrases';
EXAMPLE["<INTDE>"] = "<CALL> <INTDE> <MYCALL>";
CATEGORY["<INTQSL>"] = 'phrases';
EXAMPLE["<INTQSL>"] = "<INTQSL> 73";
CATEGORY["<INTAGNNR>"] = 'phrases';
EXAMPLE["<INTAGNNR>"] = "<INTAGNNR>";
CATEGORY["<INTAGNCALL>"] = 'phrases';
EXAMPLE["<INTAGNCALL>"] = "<INTAGNCALL>";
CATEGORY["<INTQRZ>"] = 'phrases';
EXAMPLE["<INTQRZ>"] = "<INTQRZ> de <MYCALL>";
CATEGORY["<INTGA>"] = 'phrases';
EXAMPLE["<INTGA>"] = "<INTGA> <NAME>";
CATEGORY["<INTQB4>"] = 'phrases';
EXAMPLE["<INTQB4>"] = "<INTQB4>";
CATEGORY["<INTQRL>"] = 'phrases';
EXAMPLE["<INTQRL>"] = "<INTQRL>";
CATEGORY["<INTCQ>"] = 'phrases';
EXAMPLE["<INTCQ>"] = "CQ <INTCQ>";
CATEGORY["<INTE>"] = 'phrases';
EXAMPLE["<INTE>"] = "<NAME> <INTE> <QTH>";
CATEGORY["<GM>"] = 'phrases';
EXAMPLE["<GM>"] = "<GM> <NAME>";
ALIASES["<GM>"] = ["<GE>", "<GN>"];
CATEGORY["<LANG_xx>"] = 'phrases';
EXAMPLE["<LANG_xx>"] = "<LANG_DE><INTQSL>";
CATEGORY["<FORCELANGUAGE:xx>"] = 'phrases';
EXAMPLE["<FORCELANGUAGE:xx>"] = "<FORCELANGUAGE:FR>";
CATEGORY["<TXTIME>"] = 'time';
EXAMPLE["<TXTIME>"] = "TX <TXTIME>";
CATEGORY["<LOCALDATE>"] = 'time';
EXAMPLE["<LOCALDATE>"] = "<LOCALDATE>";
CATEGORY["<TIME:formato>"] = 'time';
EXAMPLE["<TIME:formato>"] = "<TIME:%H%M>z";
ALIASES["<TIME:formato>"] = ["<DATE:formato>", "<LOCALTIME:formato>", "<CTIME:formato>"];
CATEGORY["<STARTTIME:hhmm>"] = 'time';
EXAMPLE["<STARTTIME:hhmm>"] = "<STARTTIME:1230>";
CATEGORY["<TIMER:s,macro>"] = 'time';
EXAMPLE["<TIMER:s,macro>"] = "<TIMER:600,F2>";
CATEGORY["<CLEARNRR>"] = 'qso';
EXAMPLE["<CLEARNRR>"] = "<CLEARNRR>";
CATEGORY["<INCNRR>"] = 'qso';
EXAMPLE["<INCNRR>"] = "<INCNRR>";
CATEGORY["<CORRECTCALLTX>"] = 'qso';
EXAMPLE["<CORRECTCALLTX>"] = "<CORRECTCALLTX> <CORRECTCALLTX> TU";
CATEGORY["<SAVE>"] = 'qso';
EXAMPLE["<SAVE>"] = "<SAVE>";
ALIASES["<SAVE>"] = ["<SAVESH>"];
CATEGORY["<SAVESHAUTO>"] = 'qso';
EXAMPLE["<SAVESHAUTO>"] = "TU <SAVESHAUTO>";
CATEGORY["<READLOG>"] = 'qso';
EXAMPLE["<READLOG>"] = "<READLOG>";
ALIASES["<READLOG>"] = ["<REFRESH>"];
CATEGORY["<WKD>"] = 'qso';
EXAMPLE["<WKD>"] = "<WKD>";
CATEGORY["<WKDIGNOREBEFORE:data>"] = 'qso';
EXAMPLE["<WKDIGNOREBEFORE:data>"] = "<WKDIGNOREBEFORE:2026-11-14 12:00>";
CATEGORY["<ERASE>"] = 'qso';
EXAMPLE["<ERASE>"] = "<ERASE>";
CATEGORY["<PRINTQSL>"] = 'qso';
EXAMPLE["<PRINTQSL>"] = "<PRINTQSL>";
ALIASES["<PRINTQSL>"] = ["<QSLPRINT>"];
CATEGORY["<SENDEQSL>"] = 'qso';
EXAMPLE["<SENDEQSL>"] = "<SAVEQSO><SENDEQSL>";
ALIASES["<SENDEQSL>"] = ["<SENDEQSLNP>"];
CATEGORY["<SENDSPOT:commento>"] = 'qso';
EXAMPLE["<SENDSPOT:commento>"] = "<SENDSPOT:RTTY TNX QSO>";
CATEGORY["<QSOCMDDDE:comando>"] = 'qso';
EXAMPLE["<QSOCMDDDE:comando>"] = "<QSOCMDDDE:LOG>";
CATEGORY["<LOCKMODE>"] = 'mode';
EXAMPLE["<LOCKMODE>"] = "<LOCKMODE>";
ALIASES["<LOCKMODE>"] = ["<UNLOCKMODE>"];
CATEGORY["<LOCKRX>"] = 'mode';
EXAMPLE["<LOCKRX>"] = "<LOCKRX>";
ALIASES["<LOCKRX>"] = ["<UNLOCKRX>", "<LOCKRXTOGGLE>"];
CATEGORY["<MODEADD:modo>"] = 'mode';
EXAMPLE["<MODEADD:modo>"] = "<MODEADD:RTTY>";
ALIASES["<MODEADD:modo>"] = ["<MODETNC:modo>"];
CATEGORY["<WORDMODE:ON>"] = 'mode';
EXAMPLE["<WORDMODE:ON>"] = "<WORDMODE:ON>";
CATEGORY["<CAPITALSONLY:ON>"] = 'mode';
EXAMPLE["<CAPITALSONLY:ON>"] = "<CAPITALSONLY:ON>";
CATEGORY["<CWID>"] = 'mode';
EXAMPLE["<CWID>"] = "73 <CWID><RX>";
CATEGORY["<REVERSEPADDLE>"] = 'mode';
EXAMPLE["<REVERSEPADDLE>"] = "<REVERSEPADDLE>";
CATEGORY["<SNAPNOW>"] = 'mode';
EXAMPLE["<SNAPNOW>"] = "<SNAPNOW>";
CATEGORY["<INV>"] = 'mode';
EXAMPLE["<INV>"] = "<INV>";
ALIASES["<INV>"] = ["<INVON>", "<INVOFF>"];
CATEGORY["<FILTER:PASS>"] = 'mode';
EXAMPLE["<FILTER:PASS>"] = "<FILTER:PASS>";
CATEGORY["<BROADCASTMODE:ON>"] = 'mode';
EXAMPLE["<BROADCASTMODE:ON>"] = "<BROADCASTMODE:ON>";
CATEGORY["<TOGGLECHANNEL>"] = 'mode';
EXAMPLE["<TOGGLECHANNEL>"] = "<TOGGLECHANNEL>";
CATEGORY["<ESCAPE>"] = 'txrx';
EXAMPLE["<ESCAPE>"] = "<ESCAPE>";
CATEGORY["<PTT:ON>"] = 'txrx';
EXAMPLE["<PTT:ON>"] = "<PTT:ON>";
CATEGORY["<SET BW:Hz>"] = 'mode';
EXAMPLE["<SET BW:Hz>"] = "<SET BW:250>";
ALIASES["<SET BW:Hz>"] = ["<SET TONES:n>", "<SET SUBMODE:n>"];
CATEGORY["<GET SN>"] = 'mode';
EXAMPLE["<GET SN>"] = "UR S/N <GET SN> dB";
ALIASES["<GET SN>"] = ["<GET OFFSET>", "<GET RATE>"];
CATEGORY["<SPZOOM:n>"] = 'freq';
EXAMPLE["<SPZOOM:n>"] = "<SPZOOM:2>";
CATEGORY["<SPSHIFT:Hz>"] = 'freq';
EXAMPLE["<SPSHIFT:Hz>"] = "<SPSHIFT:1500>";
CATEGORY["<AMPLITUDE:dB>"] = 'freq';
EXAMPLE["<AMPLITUDE:dB>"] = "<AMPLITUDE:3>";
CATEGORY["<ZFQ>"] = 'freq';
EXAMPLE["<ZFQ>"] = "<ZFQ>";
CATEGORY["<FQB:kHz>"] = 'freq';
EXAMPLE["<FQB:kHz>"] = "<FQB:14085.5>";
CATEGORY["<TEST>"] = 'txrx';
EXAMPLE["<TEST>"] = "CQ <TEST> <MYCALL>";
CATEGORY["<CATCMDHEX:byte>"] = 'cat';
EXAMPLE["<CATCMDHEX:byte>"] = "<CATCMDHEX:FE FE 94 E0 03 FD>";
CATEGORY["<CATCMDX:comando>"] = 'cat';
EXAMPLE["<CATCMDX:comando>"] = "<CATCMDX:FA00014070000;>";
ALIASES["<CATCMDX:comando>"] = ["<CATCMDDDE:comando>"];
CATEGORY["<CATWAIT:ms>"] = 'cat';
EXAMPLE["<CATWAIT:ms>"] = "<CATCMD:AN1;><CATWAIT:200><CATCMD:AN2;>";
CATEGORY["<CATTRANSVERTER:MHz>"] = 'cat';
EXAMPLE["<CATTRANSVERTER:MHz>"] = "<CATTRANSVERTER:116>";
CATEGORY["<CATVFOSYNCONCE>"] = 'cat';
EXAMPLE["<CATVFOSYNCONCE>"] = "<CATVFOSYNCONCE>";
ALIASES["<CATVFOSYNCONCE>"] = ["<CATVFOSYNCSHIFT:Hz>"];
CATEGORY["<CATENABLEVFOSYNC>"] = 'cat';
EXAMPLE["<CATENABLEVFOSYNC>"] = "<CATENABLEVFOSYNC>";
ALIASES["<CATENABLEVFOSYNC>"] = ["<CATDISABLEVFOSYNC>"];
CATEGORY["<TOGGLECATPORT>"] = 'cat';
EXAMPLE["<TOGGLECATPORT>"] = "<TOGGLECATPORT>";
ALIASES["<TOGGLECATPORT>"] = ["<TOGGLEPTTPORT>"];
CATEGORY["<TOGGLESOUNDDEV>"] = 'cat';
EXAMPLE["<TOGGLESOUNDDEV>"] = "<TOGGLESOUNDDEV>";
CATEGORY["<INPUTVOLUME:%>"] = 'cat';
EXAMPLE["<INPUTVOLUME:%>"] = "<INPUTVOLUME:60>";
CATEGORY["<OUTPUTVOLUME:dB>"] = 'cat';
EXAMPLE["<OUTPUTVOLUME:dB>"] = "<OUTPUTVOLUME:6>";
CATEGORY["<WKTEXT:testo>"] = 'keyer';
EXAMPLE["<WKTEXT:testo>"] = "<WKTEXT:CQ TEST>";
ALIASES["<WKTEXT:testo>"] = ["<WK:testo>"];
CATEGORY["<WKHEX:byte>"] = 'keyer';
EXAMPLE["<WKHEX:byte>"] = "<WKHEX:02 1E>";
CATEGORY["<WKPOT>"] = 'keyer';
EXAMPLE["<WKPOT>"] = "<WKPOT>";
ALIASES["<WKPOT>"] = ["<WKNOPOT>"];
CATEGORY["<ROTORCMD:comando>"] = 'rotor';
EXAMPLE["<ROTORCMD:comando>"] = "<ROTORCMD:M180>";
CATEGORY["<ROTORCMDHEX:byte>"] = 'rotor';
EXAMPLE["<ROTORCMDHEX:byte>"] = "<ROTORCMDHEX:4D 31 38 30>";
CATEGORY["<F1>"] = 'nested';
EXAMPLE["<F1>"] = "<F2> <F3>";
CATEGORY["<CALL?>"] = 'nested';
EXAMPLE["<CALL?>"] = "<CALL?> <RSTS?> <NRS?>";
ALIASES["<CALL?>"] = ["<RSTS?>", "<RSTR?>", "<NRS?>", "<NRR?>"];
CATEGORY["<F1?>"] = 'nested';
EXAMPLE["<F1?>"] = "<F5?>";

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
