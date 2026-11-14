/** L'elenco dei comandi delle macro che Aurora esegue, per la finestra di modifica: gruppo, comando e spiegazione nelle sei lingue (it, en, de, fr, es, pt). */

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
}

const c = (group: CommandGroup, tag: string, ...desc: [string, string, string, string, string, string]): MacroCommand => ({ group, tag, desc });
/** Un comando che Aurora non esegue ancora (si vede nell'elenco, con «non ancora»). */
const cs = (group: CommandGroup, tag: string, ...desc: [string, string, string, string, string, string]): MacroCommand => ({ group, tag, desc, soon: true });

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
  c('autocq', '<AUTOCQ>', 'avvia o ferma la chiamata CQ automatica', 'starts or stops the automatic CQ', 'startet oder stoppt den automatischen CQ-Ruf', 'démarre ou arrête l’appel CQ automatique', 'inicia o detiene la llamada CQ automática', 'inicia ou pára a chamada CQ automática'),
  c('program', '<TUNE>', 'manda un tono per accordare (<TUNE:n> per n secondi)', 'sends a tuning tone (<TUNE:n> for n seconds)', 'sendet einen Abstimmton (<TUNE:n> für n Sekunden)', 'envoie une porteuse d’accord (<TUNE:n> pendant n secondes)', 'envía un tono de sintonía (<TUNE:n> durante n segundos)', 'envia um tom de sintonia (<TUNE:n> durante n segundos)'),
  c('program', '<BEEP:ON>', 'suona un avviso', 'sounds a beep', 'gibt einen Signalton aus', 'émet un bip', 'emite un pitido', 'emite um aviso sonoro'),
  c('program', '<CURSOR:TX>', 'porta il cursore nella casella di trasmissione', 'puts the cursor in the transmit box', 'setzt den Cursor ins Sendefeld', 'place le curseur dans la case d’émission', 'pone el cursor en la casilla de transmisión', 'põe o cursor na caixa de transmissão'),
  c('macros', '<MACROS:file>', 'carica un altro insieme di macro', 'loads another macro set', 'lädt einen anderen Makrosatz', 'charge un autre jeu de macros', 'carga otro conjunto de macros', 'carrega outro conjunto de macros'),
  c('frequency', '<AUDIOFQ:Hz>', 'imposta la frequenza audio (con + o - è relativa)', 'sets the audio frequency (relative with + or -)', 'setzt die Audiofrequenz (mit + oder - relativ)', 'règle la fréquence audio (relative avec + ou -)', 'fija la frecuencia de audio (relativa con + o -)', 'define a frequência de áudio (relativa com + ou -)'),
  c('frequency', '<SEEKLEFT>', 'cerca un segnale verso sinistra', 'seeks a signal to the left', 'sucht ein Signal nach links', 'cherche un signal vers la gauche', 'busca una señal hacia la izquierda', 'procura um sinal para a esquerda'),
  c('frequency', '<SEEKRIGHT>', 'cerca un segnale verso destra', 'seeks a signal to the right', 'sucht ein Signal nach rechts', 'cherche un signal vers la droite', 'busca una señal hacia la derecha', 'procura um sinal para a direita'),
  c('frequency', '<BOOKMARK>', 'mette o toglie il segnalibro', 'toggles the bookmark', 'setzt oder entfernt das Lesezeichen', 'pose ou retire le signet', 'pone o quita el marcador', 'põe ou tira o marcador'),
  c('mode', '<MODE:PSK31>', 'cambia modo (PSK31, PSK63, RTTY)', 'changes mode (PSK31, PSK63, RTTY)', 'wechselt den Modus (PSK31, PSK63, RTTY)', 'change de mode (PSK31, PSK63, RTTY)', 'cambia de modo (PSK31, PSK63, RTTY)', 'muda de modo (PSK31, PSK63, RTTY)'),
  c('rtty', '<BAUDRATE:n>', 'velocità: 31.25 o 62.5 per PSK, 45.45, 50, 75... per RTTY', 'speed: 31.25 or 62.5 for PSK, 45.45, 50, 75... for RTTY', 'Geschwindigkeit: 31,25 oder 62,5 für PSK, 45,45, 50, 75 ... für RTTY', 'vitesse : 31,25 ou 62,5 pour PSK, 45,45, 50, 75... pour RTTY', 'velocidad: 31,25 o 62,5 para PSK, 45,45, 50, 75... para RTTY', 'velocidade: 31,25 ou 62,5 para PSK, 45,45, 50, 75... para RTTY'),
  c('rtty', '<SHIFT:Hz>', 'spostamento RTTY in Hz', 'RTTY shift in Hz', 'RTTY-Shift in Hz', 'shift RTTY en Hz', 'desplazamiento RTTY en Hz', 'desvio RTTY em Hz'),
  // Auto CQ
  c('autocq', '<ACQ>', 'come <AUTOCQ>', 'same as <AUTOCQ>', 'wie <AUTOCQ>', 'comme <AUTOCQ>', 'igual que <AUTOCQ>', 'igual a <AUTOCQ>'),
  c('autocq', '<ALARM:testo>', 'mostra un avviso con il testo', 'shows a notice with the text', 'zeigt einen Hinweis mit dem Text', 'affiche un avis avec le texte', 'muestra un aviso con el texto', 'mostra um aviso com o texto'),
  cs('autocq', '<ACQDELAY:s>', 'pausa fra due CQ automatici (secondi)', 'pause between two automatic CQs (seconds)', 'Pause zwischen zwei automatischen CQ (Sekunden)', 'pause entre deux CQ automatiques (secondes)', 'pausa entre dos CQ automáticos (segundos)', 'pausa entre dois CQ automáticos (segundos)'),
  cs('autocq', '<ASAUTOCQ>', 'usa il testo dell’Auto CQ come macro', 'uses the Auto CQ text as a macro', 'verwendet den Auto-CQ-Text als Makro', 'utilise le texte de l’Auto CQ comme macro', 'usa el texto del Auto CQ como macro', 'usa o texto do Auto CQ como macro'),
  cs('autocq', '<AUTOCQRESUME>', 'riprende l’Auto CQ fermato', 'resumes a stopped Auto CQ', 'setzt einen gestoppten Auto-CQ fort', 'reprend un Auto CQ arrêté', 'reanuda un Auto CQ detenido', 'retoma um Auto CQ parado'),
  // Macro
  c('macros', '<EDITMACRO:nome>', 'apre la finestra di modifica di una macro (F5, Ctrl-F5...)', 'opens the edit window of a macro (F5, Ctrl-F5...)', 'öffnet das Bearbeitungsfenster eines Makros (F5, Ctrl-F5 ...)', 'ouvre la fenêtre de modification d’une macro (F5, Ctrl-F5...)', 'abre la ventana de edición de una macro (F5, Ctrl-F5...)', 'abre a janela de edição de uma macro (F5, Ctrl-F5...)'),
  cs('macros', '<MODEMACROS:file>', 'carica le macro di un altro modo', 'loads the macros of another mode', 'lädt die Makros eines anderen Modus', 'charge les macros d’un autre mode', 'carga las macros de otro modo', 'carrega as macros de outro modo'),
  cs('macros', '<HIDETEXT>', 'nasconde il testo che si trasmette', 'hides the text being sent', 'blendet den gesendeten Text aus', 'masque le texte émis', 'oculta el texto que se transmite', 'oculta o texto que se transmite'),
  cs('macros', '<SHOWTEXT>', 'mostra di nuovo il testo che si trasmette', 'shows the text being sent again', 'zeigt den gesendeten Text wieder', 'réaffiche le texte émis', 'vuelve a mostrar el texto que se transmite', 'volta a mostrar o texto que se transmite'),
  // Frequenza
  c('frequency', '<FQ:kHz>', 'porta la radio in modo che il segnale cada su questa frequenza in kHz', 'tunes the radio so the signal falls on this frequency in kHz', 'stellt das Funkgerät so ein, dass das Signal auf dieser Frequenz in kHz liegt', 'règle la radio pour que le signal tombe sur cette fréquence en kHz', 'sintoniza la radio para que la señal caiga en esta frecuencia en kHz', 'sintoniza o rádio para que o sinal caia nesta frequência em kHz'),
  c('frequency', '<SPLIT:1>', 'split acceso (1) o spento (0): la trasmissione resta dov’è mentre sposti l’ascolto', 'split on (1) or off (0): the transmit stays put while you move the receive', 'Split an (1) oder aus (0): das Senden bleibt, während du den Empfang verschiebst', 'split activé (1) ou coupé (0) : l’émission reste en place pendant que tu déplaces la réception', 'split activado (1) o apagado (0): la transmisión se queda mientras mueves la recepción', 'split ligado (1) ou desligado (0): a transmissão fica onde está enquanto moves a receção'),
  cs('frequency', '<JUMPTOTXFQ>', 'porta l’ascolto sulla frequenza di trasmissione', 'moves the receive to the transmit frequency', 'setzt den Empfang auf die Sendefrequenz', 'place la réception sur la fréquence d’émission', 'lleva la recepción a la frecuencia de transmisión', 'leva a receção para a frequência de transmissão'),
  cs('frequency', '<ALIGN:Hz>', 'allinea la frequenza di ricezione a un multiplo di questi Hz', 'aligns the receive frequency to a multiple of these Hz', 'richtet die Empfangsfrequenz an einem Vielfachen dieser Hz aus', 'aligne la fréquence de réception sur un multiple de ces Hz', 'alinea la frecuencia de recepción a un múltiplo de estos Hz', 'alinha a frequência de receção a um múltiplo destes Hz'),
  cs('frequency', '<CATCMD:comando>', 'manda un comando CAT alla radio', 'sends a CAT command to the radio', 'sendet einen CAT-Befehl an das Funkgerät', 'envoie une commande CAT à la radio', 'envía un comando CAT a la radio', 'envia um comando CAT ao rádio'),
  // Impostazioni del modo
  c('mode', '<AFCON>', 'accende l’AFC (<AFCOFF> lo spegne, <AFC> lo alterna)', 'turns AFC on (<AFCOFF> off, <AFC> toggles)', 'schaltet AFC ein (<AFCOFF> aus, <AFC> wechselt)', 'active l’AFC (<AFCOFF> le coupe, <AFC> bascule)', 'activa el AFC (<AFCOFF> lo apaga, <AFC> alterna)', 'liga o AFC (<AFCOFF> desliga, <AFC> alterna)'),
  c('mode', '<SQUELCHON>', 'accende lo squelch (<SQUELCHOFF>, <SQUELCH>)', 'turns squelch on (<SQUELCHOFF>, <SQUELCH>)', 'schaltet Squelch ein (<SQUELCHOFF>, <SQUELCH>)', 'active le squelch (<SQUELCHOFF>, <SQUELCH>)', 'activa el squelch (<SQUELCHOFF>, <SQUELCH>)', 'liga o squelch (<SQUELCHOFF>, <SQUELCH>)'),
  c('mode', '<SNAPON>', 'accende Snap (<SNAPOFF>, <SNAP>)', 'turns Snap on (<SNAPOFF>, <SNAP>)', 'schaltet Snap ein (<SNAPOFF>, <SNAP>)', 'active Snap (<SNAPOFF>, <SNAP>)', 'activa Snap (<SNAPOFF>, <SNAP>)', 'liga o Snap (<SNAPOFF>, <SNAP>)'),
  c('mode', '<LOCKTX>', 'blocca la trasmissione dov’è (<UNLOCKTX> la libera)', 'locks the transmit where it is (<UNLOCKTX> frees it)', 'hält das Senden fest (<UNLOCKTX> gibt es frei)', 'fixe l’émission où elle est (<UNLOCKTX> la libère)', 'fija la transmisión donde está (<UNLOCKTX> la libera)', 'fixa a transmissão onde está (<UNLOCKTX> liberta-a)'),
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

/** Le voci di un gruppo (tutte se `group` e' vuoto). */
export function commandsOf(group: CommandGroup | ''): readonly MacroCommand[] {
  return group === '' ? MACRO_COMMANDS : MACRO_COMMANDS.filter((x) => x.group === group);
}
