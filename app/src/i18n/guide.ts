import type { Entry } from './core';

/**
 * La guida: capitoli, ognuno con argomenti fatti di passi numerati (cosa premere, dove) e un consiglio. I nomi di schede e pulsanti non
 * si riscrivono: `[[chiave]]` prende il testo vero dell'interfaccia nella lingua scelta, cosi' la guida dice sempre cio' che si vede.
 */
export interface GuideSection {
  id: string;
  title: Entry;
  /** Una riga di spiegazione prima dei passi. */
  text?: Entry;
  steps?: readonly Entry[];
  /** Un consiglio o un avvertimento, in un riquadro a parte. */
  tip?: Entry;
}

export interface GuideChapter {
  id: string;
  title: Entry;
  sections: readonly GuideSection[];
}

export const GUIDE: readonly GuideChapter[] = [
  {
    id: 'start',
    title: ['Primi passi', 'Getting started', 'Erste Schritte', 'Premiers pas', 'Primeros pasos', 'Primeiros passos'],
    sections: [
      {
        id: 'station',
        title: ['La tua stazione', 'Your station', 'Deine Station', 'Ta station', 'Tu estación', 'A tua estação'],
        steps: [
          ['Apri [[nav.settings]] → [[tab.general]].', 'Open [[nav.settings]] → [[tab.general]].', 'Öffne [[nav.settings]] → [[tab.general]].', 'Ouvre [[nav.settings]] → [[tab.general]].', 'Abre [[nav.settings]] → [[tab.general]].', 'Abre [[nav.settings]] → [[tab.general]].'],
          ['Scrivi il tuo nominativo e il tuo locatore (6 caratteri, per esempio JN55ab).', 'Enter your callsign and your locator (6 characters, e.g. JN55ab).', 'Trage Rufzeichen und Locator ein (6 Zeichen, z. B. JN55ab).', 'Saisis ton indicatif et ton locator (6 caractères, par ex. JN55ab).', 'Escribe tu indicativo y tu localizador (6 caracteres, por ejemplo JN55ab).', 'Escreve o teu indicativo e o teu locator (6 caracteres, por exemplo JN55ab).'],
          ['Il locatore serve per le distanze, la mappa e il rapporto delle stazioni che ti sentono.', 'The locator is used for distances, the map and the stations that hear you.', 'Der Locator dient für Entfernungen, Karte und die Stationen, die dich hören.', 'Le locator sert aux distances, à la carte et aux stations qui t’entendent.', 'El localizador sirve para distancias, el mapa y las estaciones que te oyen.', 'O locator serve para distâncias, mapa e estações que te ouvem.'],
        ],
      },
      {
        id: 'audio',
        title: ['La scheda audio', 'The sound card', 'Die Soundkarte', 'La carte son', 'La tarjeta de sonido', 'A placa de som'],
        steps: [
          ['Apri [[nav.settings]] → [[tab.audio]].', 'Open [[nav.settings]] → [[tab.audio]].', 'Öffne [[nav.settings]] → [[tab.audio]].', 'Ouvre [[nav.settings]] → [[tab.audio]].', 'Abre [[nav.settings]] → [[tab.audio]].', 'Abre [[nav.settings]] → [[tab.audio]].'],
          ['Ingresso: la scheda che riceve l’audio dalla radio (di solito «Microfono (USB Audio Device)»).', 'Input: the card that receives the audio from the radio (usually “Microphone (USB Audio Device)”).', 'Eingang: die Karte, die das Audio vom Funkgerät bekommt (meist „Mikrofon (USB Audio Device)“).', 'Entrée : la carte qui reçoit le son de la radio (en général « Microphone (USB Audio Device) »).', 'Entrada: la tarjeta que recibe el audio de la radio (normalmente «Micrófono (USB Audio Device)»).', 'Entrada: a placa que recebe o áudio do rádio (normalmente «Microfone (USB Audio Device)»).'],
          ['Uscita: la scheda che manda l’audio alla radio per trasmettere (di solito «Altoparlanti (USB Audio Device)»).', 'Output: the card that sends audio to the radio to transmit (usually “Speakers (USB Audio Device)”).', 'Ausgang: die Karte, die zum Senden Audio an das Funkgerät schickt (meist „Lautsprecher (USB Audio Device)“).', 'Sortie : la carte qui envoie le son à la radio pour émettre (en général « Haut-parleurs (USB Audio Device) »).', 'Salida: la tarjeta que envía el audio a la radio para transmitir (normalmente «Altavoces (USB Audio Device)»).', 'Saída: a placa que envia o áudio para o rádio para transmitir (normalmente «Altifalantes (USB Audio Device)»).'],
          ['Sulla pagina principale la barra Rx deve dire «Giusto». Se dice Basso o Saturo, sposta il cursore Rx (o lascia AUTO acceso).', 'On the main page the Rx bar should say “Good”. If it says Low or Overdriven, move the Rx slider (or keep AUTO on).', 'Auf der Hauptseite sollte die Rx-Leiste „Gut“ zeigen. Bei Niedrig oder Übersteuert den Rx-Regler bewegen (oder AUTO an lassen).', 'Sur la page principale, la barre Rx doit indiquer « Bon ». Si elle indique Faible ou Saturé, bouge le curseur Rx (ou laisse AUTO).', 'En la página principal la barra Rx debe decir «Correcto». Si dice Bajo o Saturado, mueve el cursor Rx (o deja AUTO).', 'Na página principal a barra Rx deve dizer «Bom». Se disser Baixo ou Saturado, move o cursor Rx (ou deixa AUTO).'],
        ],
      },
      {
        id: 'radio',
        title: ['Collegare la radio (CAT)', 'Connecting the radio (CAT)', 'Funkgerät verbinden (CAT)', 'Connecter la radio (CAT)', 'Conectar la radio (CAT)', 'Ligar o rádio (CAT)'],
        steps: [
          ['Apri [[nav.settings]] → [[tab.radio]].', 'Open [[nav.settings]] → [[tab.radio]].', 'Öffne [[nav.settings]] → [[tab.radio]].', 'Ouvre [[nav.settings]] → [[tab.radio]].', 'Abre [[nav.settings]] → [[tab.radio]].', 'Abre [[nav.settings]] → [[tab.radio]].'],
          ['[[cat.backend]]: scegli Hamlib. Con Hamlib arrivano anche S-meter, potenza, ROS e ALC veri dalla radio.', '[[cat.backend]]: choose Hamlib. With Hamlib you also get the real S-meter, power, SWR and ALC from the radio.', '[[cat.backend]]: Hamlib wählen. Mit Hamlib kommen auch echtes S-Meter, Leistung, SWR und ALC vom Funkgerät.', '[[cat.backend]] : choisis Hamlib. Avec Hamlib, la radio fournit aussi le vrai S-mètre, la puissance, le ROS et l’ALC.', '[[cat.backend]]: elige Hamlib. Con Hamlib llegan también el S-meter, la potencia, la ROE y el ALC reales de la radio.', '[[cat.backend]]: escolhe Hamlib. Com Hamlib chegam também o S-meter, a potência, a ROE e o ALC reais do rádio.'],
          ['Scrivi il modello nella ricerca (per esempio FT-2000) e cliccalo nella lista che compare.', 'Type the model in the search (e.g. FT-2000) and click it in the list that appears.', 'Gib das Modell in die Suche ein (z. B. FT-2000) und klicke es in der Liste an.', 'Tape le modèle dans la recherche (par ex. FT-2000) et clique-le dans la liste.', 'Escribe el modelo en la búsqueda (por ejemplo FT-2000) y pulsa en la lista que aparece.', 'Escreve o modelo na pesquisa (por exemplo FT-2000) e clica nele na lista.'],
          ['[[cat.port]]: la porta COM della radio. [[cat.baud]]: la stessa del menu CAT della radio.', '[[cat.port]]: the radio COM port. [[cat.baud]]: the same as in the radio CAT menu.', '[[cat.port]]: der COM-Port des Funkgeräts. [[cat.baud]]: wie im CAT-Menü des Funkgeräts.', '[[cat.port]] : le port COM de la radio. [[cat.baud]] : la même que dans le menu CAT de la radio.', '[[cat.port]]: el puerto COM de la radio. [[cat.baud]]: la misma del menú CAT de la radio.', '[[cat.port]]: a porta COM do rádio. [[cat.baud]]: a mesma do menu CAT do rádio.'],
          ['Premi [[cat.apply]], poi [[cat.connect]]. Lo stato deve diventare «Collegata» e compare la frequenza della radio.', 'Press [[cat.apply]], then [[cat.connect]]. The state must become “Connected” and the radio frequency appears.', 'Drücke [[cat.apply]], dann [[cat.connect]]. Der Status muss „Verbunden“ zeigen und die Frequenz erscheint.', 'Appuie sur [[cat.apply]], puis [[cat.connect]]. L’état doit devenir « Connectée » et la fréquence apparaît.', 'Pulsa [[cat.apply]] y luego [[cat.connect]]. El estado debe pasar a «Conectada» y aparece la frecuencia.', 'Carrega em [[cat.apply]] e depois [[cat.connect]]. O estado deve ficar «Ligado» e aparece a frequência.'],
        ],
        tip: ['Non sai porta e velocità? Prova «Rileva la mia radio»: le cerca da solo. Se usavi OmniRig, le trovi nella sua finestra di impostazioni.', 'Don’t know port and speed? Try “Detect my radio”: it finds them for you. If you used OmniRig, they are in its settings window.', 'Port und Geschwindigkeit unbekannt? „Mein Funkgerät erkennen“ sucht sie selbst. Mit OmniRig stehen sie in dessen Einstellungsfenster.', 'Tu ne connais pas le port et la vitesse ? Essaie « Détecter ma radio ». Si tu utilisais OmniRig, ils sont dans sa fenêtre de réglages.', '¿No sabes el puerto y la velocidad? Prueba «Detectar mi radio». Si usabas OmniRig, están en su ventana de ajustes.', 'Não sabes a porta e a velocidade? Experimenta «Detetar o meu rádio». Se usavas o OmniRig, estão na janela de definições dele.'],
      },
    ],
  },
  {
    id: 'receive',
    title: ['Ricevere', 'Receiving', 'Empfangen', 'Recevoir', 'Recibir', 'Receber'],
    sections: [
      {
        id: 'list',
        title: ['L’elenco dei messaggi', 'The message list', 'Die Nachrichtenliste', 'La liste des messages', 'La lista de mensajes', 'A lista de mensagens'],
        text: ['Ogni riga è un messaggio decodificato: ora, rapporto, testo, paese con bandiera e distanza. Le etichette a destra dicono cosa ha di speciale.', 'Each row is a decoded message: time, report, text, country with flag and distance. The tags on the right say what is special about it.', 'Jede Zeile ist eine dekodierte Nachricht: Zeit, Rapport, Text, Land mit Flagge und Entfernung. Die Marken rechts zeigen, was besonders ist.', 'Chaque ligne est un message décodé : heure, rapport, texte, pays avec drapeau et distance. Les étiquettes à droite disent ce qu’il a de spécial.', 'Cada fila es un mensaje decodificado: hora, informe, texto, país con bandera y distancia. Las etiquetas a la derecha dicen qué tiene de especial.', 'Cada linha é uma mensagem descodificada: hora, relatório, texto, país com bandeira e distância. As etiquetas à direita dizem o que tem de especial.'],
        steps: [
          ['Linguetta rossa: quella stazione ti sta chiamando.', 'Red tab: that station is calling you.', 'Rote Lasche: diese Station ruft dich.', 'Languette rouge : cette station t’appelle.', 'Pestaña roja: esa estación te está llamando.', 'Lingueta vermelha: essa estação está a chamar-te.'],
          ['Linguetta viola: quella stazione ha risposto alla tua chiamata.', 'Purple tab: that station answered your call.', 'Lila Lasche: diese Station hat auf deinen Anruf geantwortet.', 'Languette violette : cette station a répondu à ton appel.', 'Pestaña morada: esa estación ha respondido a tu llamada.', 'Lingueta roxa: essa estação respondeu à tua chamada.'],
          ['Riga barrata: stazione già collegata.', 'Struck-through row: station already worked.', 'Durchgestrichene Zeile: Station schon gearbeitet.', 'Ligne barrée : station déjà contactée.', 'Fila tachada: estación ya trabajada.', 'Linha riscada: estação já contactada.'],
          ['[[ctl.erase]] svuota l’elenco.', '[[ctl.erase]] clears the list.', '[[ctl.erase]] leert die Liste.', '[[ctl.erase]] vide la liste.', '[[ctl.erase]] vacía la lista.', '[[ctl.erase]] limpa a lista.'],
        ],
      },
      {
        id: 'spectrum',
        title: ['La cascata', 'The waterfall', 'Der Wasserfall', 'La cascade', 'La cascada', 'A cascata'],
        steps: [
          ['Ogni traccia è una stazione. La barra verde è la tua frequenza di ricezione (Rx), quella rossa di trasmissione (Tx).', 'Each trace is a station. The green bar is your receive frequency (Rx), the red one your transmit frequency (Tx).', 'Jede Spur ist eine Station. Der grüne Balken ist deine Empfangsfrequenz (Rx), der rote die Sendefrequenz (Tx).', 'Chaque trace est une station. La barre verte est ta fréquence de réception (Rx), la rouge celle d’émission (Tx).', 'Cada traza es una estación. La barra verde es tu frecuencia de recepción (Rx), la roja la de transmisión (Tx).', 'Cada traço é uma estação. A barra verde é a tua frequência de receção (Rx), a vermelha a de transmissão (Tx).'],
          ['Un clic sposta la barra verde; Maiusc + clic sposta la rossa. Scegli per trasmettere un punto libero.', 'A click moves the green bar; Shift + click moves the red one. Pick a free spot to transmit.', 'Ein Klick verschiebt den grünen Balken, Umschalt + Klick den roten. Zum Senden eine freie Stelle wählen.', 'Un clic déplace la barre verte ; Maj + clic déplace la rouge. Choisis un endroit libre pour émettre.', 'Un clic mueve la barra verde; Mayús + clic mueve la roja. Elige un hueco libre para transmitir.', 'Um clique move a barra verde; Shift + clique move a vermelha. Escolhe um sítio livre para transmitir.'],
        ],
      },
      {
        id: 'deaf',
        title: ['Vedo segnali ma non decodifica', 'I see signals but nothing decodes', 'Ich sehe Signale, aber nichts wird dekodiert', 'Je vois des signaux mais rien ne se décode', 'Veo señales pero no decodifica', 'Vejo sinais mas nada descodifica'],
        steps: [
          ['Spegni [[ctl.filter]]: acceso decodifica solo ±60 Hz attorno a Rx. Quando è acceso la cascata è scura ai lati.', 'Turn [[ctl.filter]] off: when on it decodes only ±60 Hz around Rx. When it is on the waterfall is dark at the sides.', 'Schalte [[ctl.filter]] aus: eingeschaltet dekodiert es nur ±60 Hz um Rx. Dann ist der Wasserfall an den Seiten dunkel.', 'Coupe [[ctl.filter]] : actif, il ne décode que ±60 Hz autour de Rx. La cascade est alors sombre sur les côtés.', 'Apaga [[ctl.filter]]: encendido solo decodifica ±60 Hz alrededor de Rx. Entonces la cascada está oscura a los lados.', 'Desliga [[ctl.filter]]: ligado só descodifica ±60 Hz à volta de Rx. Nesse caso a cascata fica escura dos lados.'],
          ['Controlla che [[ctl.monitor]] sia acceso.', 'Check that [[ctl.monitor]] is on.', 'Prüfe, dass [[ctl.monitor]] eingeschaltet ist.', 'Vérifie que [[ctl.monitor]] est actif.', 'Comprueba que [[ctl.monitor]] esté encendido.', 'Confirma que [[ctl.monitor]] está ligado.'],
          ['Controlla la barra Rx: deve dire «Giusto».', 'Check the Rx bar: it should say “Good”.', 'Prüfe die Rx-Leiste: sie sollte „Gut“ zeigen.', 'Vérifie la barre Rx : elle doit indiquer « Bon ».', 'Comprueba la barra Rx: debe decir «Correcto».', 'Verifica a barra Rx: deve dizer «Bom».'],
          ['Controlla l’orologio: se l’ora è sbagliata compare un avviso giallo in alto.', 'Check the clock: if the time is wrong a yellow warning appears at the top.', 'Prüfe die Uhr: ist die Zeit falsch, erscheint oben eine gelbe Warnung.', 'Vérifie l’horloge : si l’heure est fausse, un avertissement jaune apparaît en haut.', 'Comprueba el reloj: si la hora está mal aparece un aviso amarillo arriba.', 'Verifica o relógio: se a hora estiver errada aparece um aviso amarelo em cima.'],
        ],
      },
      {
        id: 'agc',
        title: ['[[ctl.agc]]: segnali deboli vicino a stazioni forti', '[[ctl.agc]]: weak signals near strong stations', '[[ctl.agc]]: schwache Signale neben starken Stationen', '[[ctl.agc]] : signaux faibles près de stations fortes', '[[ctl.agc]]: señales débiles cerca de estaciones fuertes', '[[ctl.agc]]: sinais fracos perto de estações fortes'],
        text: ['Quando una stazione forte parte o smette, l’AGC della radio cambia il volume e i segnali deboli in quel momento si perdono. [[ctl.agc]] riporta quei tratti al livello giusto. Tienilo acceso.', 'When a strong station starts or stops, the radio AGC changes the volume and weak signals at that moment are lost. [[ctl.agc]] brings those stretches back to the right level. Keep it on.', 'Wenn eine starke Station beginnt oder endet, ändert die AGC des Funkgeräts die Lautstärke und schwache Signale gehen in dem Moment verloren. [[ctl.agc]] gleicht das aus. Eingeschaltet lassen.', 'Quand une station forte commence ou s’arrête, l’AGC de la radio change le volume et les signaux faibles à ce moment se perdent. [[ctl.agc]] corrige ces passages. Laisse-le actif.', 'Cuando una estación fuerte empieza o termina, el AGC de la radio cambia el volumen y las señales débiles en ese momento se pierden. [[ctl.agc]] lo corrige. Déjalo encendido.', 'Quando uma estação forte começa ou acaba, o AGC do rádio muda o volume e os sinais fracos nesse momento perdem-se. [[ctl.agc]] corrige isso. Deixa-o ligado.'],
      },
    ],
  },
  {
    id: 'qso',
    title: ['Fare un QSO', 'Making a QSO', 'Ein QSO fahren', 'Faire un QSO', 'Hacer un QSO', 'Fazer um QSO'],
    sections: [
      {
        id: 'answer',
        title: ['Rispondere a un CQ', 'Answering a CQ', 'Auf einen CQ antworten', 'Répondre à un CQ', 'Responder a un CQ', 'Responder a um CQ'],
        steps: [
          ['Tieni acceso [[ctl.auto_tx]].', 'Keep [[ctl.auto_tx]] on.', 'Lass [[ctl.auto_tx]] eingeschaltet.', 'Garde [[ctl.auto_tx]] actif.', 'Mantén [[ctl.auto_tx]] encendido.', 'Mantém [[ctl.auto_tx]] ligado.'],
          ['Doppio clic sulla riga del CQ: Aurora prepara i messaggi e trasmette nel periodo giusto.', 'Double-click the CQ row: Aurora prepares the messages and transmits in the right period.', 'Doppelklick auf die CQ-Zeile: Aurora bereitet die Nachrichten vor und sendet im richtigen Durchgang.', 'Double-clic sur la ligne du CQ : Aurora prépare les messages et émet dans la bonne période.', 'Doble clic en la fila del CQ: Aurora prepara los mensajes y transmite en el periodo correcto.', 'Duplo clique na linha do CQ: a Aurora prepara as mensagens e transmite no período certo.'],
          ['La sequenza va avanti da sola: rapporto, R-rapporto, RR73. Quando ricevi il rapporto il QSO viene registrato.', 'The sequence goes on by itself: report, R-report, RR73. When you receive the report the QSO is logged.', 'Der Ablauf läuft von selbst: Rapport, R-Rapport, RR73. Mit dem Rapport wird das QSO geloggt.', 'La séquence avance seule : rapport, R-rapport, RR73. Le QSO est enregistré quand tu reçois le rapport.', 'La secuencia avanza sola: informe, R-informe, RR73. Al recibir el informe el QSO se registra.', 'A sequência avança sozinha: relatório, R-relatório, RR73. Quando recebes o relatório o QSO fica registado.'],
        ],
        tip: ['Una stazione che ti chiama mentre sei impegnato: tasto destro sulla sua riga e va in coda. Le rispondi appena finito il QSO.', 'A station calls you while you are busy: right-click its row and it is queued. You answer it as soon as the QSO ends.', 'Ruft dich eine Station, während du beschäftigt bist: Rechtsklick auf ihre Zeile, dann kommt sie in die Warteschlange.', 'Une station t’appelle pendant un QSO : clic droit sur sa ligne, elle passe en file d’attente.', 'Una estación te llama mientras estás ocupado: clic derecho en su fila y queda en cola.', 'Uma estação chama-te enquanto estás ocupado: clique direito na linha dela e fica em fila.'],
      },
      {
        id: 'cq',
        title: ['Chiamare CQ', 'Calling CQ', 'CQ rufen', 'Lancer appel', 'Llamar CQ', 'Chamar CQ'],
        steps: [
          ['Scegli un punto libero nella cascata con Maiusc + clic (barra rossa).', 'Pick a free spot in the waterfall with Shift + click (red bar).', 'Wähle mit Umschalt + Klick eine freie Stelle im Wasserfall (roter Balken).', 'Choisis un endroit libre dans la cascade avec Maj + clic (barre rouge).', 'Elige un hueco libre en la cascada con Mayús + clic (barra roja).', 'Escolhe um sítio livre na cascata com Shift + clique (barra vermelha).'],
          ['Premi [[tx.cq]]. Chi ti risponde viene servito in automatico.', 'Press [[tx.cq]]. Whoever answers is handled automatically.', 'Drücke [[tx.cq]]. Wer antwortet, wird automatisch bedient.', 'Appuie sur [[tx.cq]]. Celui qui répond est traité automatiquement.', 'Pulsa [[tx.cq]]. Quien responda se atiende automáticamente.', 'Carrega em [[tx.cq]]. Quem responder é atendido automaticamente.'],
          ['A QSO finito il CQ riparte da solo solo se [[ctl.auto_tx]] è acceso.', 'After the QSO, CQ restarts by itself only if [[ctl.auto_tx]] is on.', 'Nach dem QSO startet CQ nur von selbst, wenn [[ctl.auto_tx]] an ist.', 'Après le QSO, le CQ ne repart seul que si [[ctl.auto_tx]] est actif.', 'Tras el QSO, el CQ solo vuelve a empezar si [[ctl.auto_tx]] está encendido.', 'Depois do QSO, o CQ só recomeça sozinho se [[ctl.auto_tx]] estiver ligado.'],
        ],
      },
      {
        id: 'stop',
        title: ['Fermare la trasmissione', 'Stopping transmission', 'Senden stoppen', 'Arrêter l’émission', 'Detener la transmisión', 'Parar a transmissão'],
        steps: [
          ['STOP (in alto a destra) o il tasto Esc fermano subito la trasmissione.', 'STOP (top right) or the Esc key stop transmitting immediately.', 'STOP (oben rechts) oder die Esc-Taste stoppen sofort.', 'STOP (en haut à droite) ou la touche Échap arrêtent l’émission tout de suite.', 'STOP (arriba a la derecha) o la tecla Esc detienen la transmisión al momento.', 'STOP (em cima à direita) ou a tecla Esc param a transmissão logo.'],
          ['Una stazione che non manda il 73 finale: Aurora ripete RR73 al massimo il numero di volte scelto in [[nav.settings]] → [[tab.sequence]], poi si ferma.', 'A station that never sends the final 73: Aurora repeats RR73 at most the number of times set in [[nav.settings]] → [[tab.sequence]], then stops.', 'Eine Station schickt kein abschließendes 73: Aurora wiederholt RR73 höchstens so oft wie in [[nav.settings]] → [[tab.sequence]] eingestellt.', 'Une station n’envoie jamais le 73 final : Aurora répète RR73 au plus le nombre de fois réglé dans [[nav.settings]] → [[tab.sequence]].', 'Una estación no manda el 73 final: Aurora repite RR73 como máximo las veces elegidas en [[nav.settings]] → [[tab.sequence]].', 'Uma estação nunca envia o 73 final: a Aurora repete RR73 no máximo as vezes escolhidas em [[nav.settings]] → [[tab.sequence]].'],
        ],
      },
    ],
  },
  {
    id: 'power',
    title: ['Potenza e strumenti', 'Power and meters', 'Leistung und Anzeigen', 'Puissance et instruments', 'Potencia e instrumentos', 'Potência e instrumentos'],
    sections: [
      {
        id: 'pwr',
        title: ['Il cursore Pwr', 'The Pwr slider', 'Der Pwr-Regler', 'Le curseur Pwr', 'El cursor Pwr', 'O cursor Pwr'],
        text: ['Pwr è il volume di Windows dell’uscita audio verso la radio: 100% sul cursore = 100% nel mixer di Windows. Aurora lo riallinea prima di ogni trasmissione.', 'Pwr is the Windows volume of the audio output to the radio: 100% on the slider = 100% in the Windows mixer. Aurora realigns it before every transmission.', 'Pwr ist die Windows-Lautstärke des Audioausgangs zum Funkgerät: 100 % am Regler = 100 % im Windows-Mixer. Aurora gleicht sie vor jedem Senden ab.', 'Pwr est le volume Windows de la sortie audio vers la radio : 100 % sur le curseur = 100 % dans le mélangeur Windows. Aurora le réaligne avant chaque émission.', 'Pwr es el volumen de Windows de la salida de audio hacia la radio: 100 % en el cursor = 100 % en el mezclador de Windows. Aurora lo realinea antes de cada transmisión.', 'Pwr é o volume do Windows da saída de áudio para o rádio: 100 % no cursor = 100 % no misturador do Windows. A Aurora realinha-o antes de cada transmissão.'],
        steps: [
          ['Trasmetti e guarda l’ALC della radio.', 'Transmit and watch the radio ALC.', 'Sende und beobachte die ALC des Funkgeräts.', 'Émets et regarde l’ALC de la radio.', 'Transmite y mira el ALC de la radio.', 'Transmite e olha para o ALC do rádio.'],
          ['Abbassa Pwr finché l’ALC sta a zero o quasi. La potenza la scegli con la manopola della radio, non con Pwr.', 'Lower Pwr until the ALC is at or near zero. Choose the power with the radio knob, not with Pwr.', 'Senke Pwr, bis die ALC bei null oder fast null ist. Die Leistung stellst du am Funkgerät ein, nicht mit Pwr.', 'Baisse Pwr jusqu’à ce que l’ALC soit à zéro ou presque. Règle la puissance avec le bouton de la radio, pas avec Pwr.', 'Baja Pwr hasta que el ALC quede en cero o casi. La potencia se elige con el mando de la radio, no con Pwr.', 'Baixa o Pwr até o ALC ficar a zero ou quase. A potência escolhe-se no botão do rádio, não no Pwr.'],
        ],
      },
      {
        id: 'meters',
        title: ['S-meter e potenza sul display', 'S-meter and power on the display', 'S-Meter und Leistung im Display', 'S-mètre et puissance sur l’afficheur', 'S-meter y potencia en la pantalla', 'S-meter e potência no visor'],
        steps: [
          ['Con Hamlib il display mostra i valori veri della radio: S-meter in ricezione, potenza in trasmissione.', 'With Hamlib the display shows the real radio values: S-meter on receive, power on transmit.', 'Mit Hamlib zeigt das Display die echten Werte: S-Meter beim Empfang, Leistung beim Senden.', 'Avec Hamlib l’afficheur montre les vraies valeurs : S-mètre en réception, puissance en émission.', 'Con Hamlib la pantalla muestra los valores reales: S-meter en recepción, potencia en transmisión.', 'Com Hamlib o visor mostra os valores reais: S-meter na receção, potência na transmissão.'],
          ['Una tilde davanti al valore (~S3) vuol dire stima dall’audio: la radio non manda l’S-meter (per esempio con OmniRig).', 'A tilde before the value (~S3) means an estimate from the audio: the radio does not send the S-meter (e.g. with OmniRig).', 'Eine Tilde vor dem Wert (~S3) bedeutet Schätzung aus dem Audio: das Funkgerät liefert kein S-Meter (z. B. mit OmniRig).', 'Un tilde devant la valeur (~S3) signifie une estimation depuis l’audio : la radio n’envoie pas le S-mètre (par ex. avec OmniRig).', 'Una tilde delante del valor (~S3) significa estimación desde el audio: la radio no envía el S-meter (por ejemplo con OmniRig).', 'Um til antes do valor (~S3) significa estimativa a partir do áudio: o rádio não envia o S-meter (por exemplo com OmniRig).'],
          ['In FT8 la potenza resta ferma per tutta la trasmissione: è normale.', 'In FT8 the power stays steady for the whole transmission: that is normal.', 'Bei FT8 bleibt die Leistung während des ganzen Sendens gleich: das ist normal.', 'En FT8 la puissance reste stable pendant toute l’émission : c’est normal.', 'En FT8 la potencia se mantiene fija durante toda la transmisión: es normal.', 'Em FT8 a potência fica parada durante toda a transmissão: é normal.'],
        ],
      },
    ],
  },
  {
    id: 'share',
    title: ['Aurora con altri programmi', 'Aurora with other programs', 'Aurora mit anderen Programmen', 'Aurora avec d’autres programmes', 'Aurora con otros programas', 'Aurora com outros programas'],
    sections: [
      {
        id: 'sharing',
        title: ['Una radio, più programmi', 'One radio, several programs', 'Ein Funkgerät, mehrere Programme', 'Une radio, plusieurs programmes', 'Una radio, varios programas', 'Um rádio, vários programas'],
        text: ['Due programmi che comandano la radio insieme si disturbano. Meglio che solo Aurora parli con la radio e gli altri chiedano ad Aurora.', 'Two programs driving the radio at once disturb each other. Better if only Aurora talks to the radio and the others ask Aurora.', 'Zwei Programme, die gleichzeitig das Funkgerät steuern, stören sich. Besser spricht nur Aurora mit dem Funkgerät und die anderen fragen Aurora.', 'Deux programmes qui pilotent la radio en même temps se gênent. Mieux vaut que seule Aurora parle à la radio et que les autres demandent à Aurora.', 'Dos programas que manejan la radio a la vez se molestan. Mejor que solo Aurora hable con la radio y los demás pregunten a Aurora.', 'Dois programas a comandar o rádio ao mesmo tempo atrapalham-se. É melhor só a Aurora falar com o rádio e os outros perguntarem à Aurora.'],
        steps: [
          ['In [[nav.settings]] → [[tab.radio]] accendi «[[cat.share]]» (porta 4532) e premi [[cat.apply]].', 'In [[nav.settings]] → [[tab.radio]] turn on “[[cat.share]]” (port 4532) and press [[cat.apply]].', 'In [[nav.settings]] → [[tab.radio]] „[[cat.share]]“ (Port 4532) einschalten und [[cat.apply]] drücken.', 'Dans [[nav.settings]] → [[tab.radio]], active « [[cat.share]] » (port 4532) et appuie sur [[cat.apply]].', 'En [[nav.settings]] → [[tab.radio]] activa «[[cat.share]]» (puerto 4532) y pulsa [[cat.apply]].', 'Em [[nav.settings]] → [[tab.radio]] liga «[[cat.share]]» (porta 4532) e carrega em [[cat.apply]].'],
          ['Nell’altro programma scegli Hamlib in rete (NET rigctl), indirizzo 127.0.0.1, porta 4532.', 'In the other program choose network Hamlib (NET rigctl), address 127.0.0.1, port 4532.', 'Im anderen Programm Hamlib über Netzwerk (NET rigctl) wählen, Adresse 127.0.0.1, Port 4532.', 'Dans l’autre programme, choisis Hamlib réseau (NET rigctl), adresse 127.0.0.1, port 4532.', 'En el otro programa elige Hamlib en red (NET rigctl), dirección 127.0.0.1, puerto 4532.', 'No outro programa escolhe Hamlib em rede (NET rigctl), endereço 127.0.0.1, porta 4532.'],
          ['Apri sempre prima Aurora, poi gli altri programmi.', 'Always open Aurora first, then the other programs.', 'Öffne immer zuerst Aurora, dann die anderen Programme.', 'Ouvre toujours Aurora en premier, puis les autres programmes.', 'Abre siempre primero Aurora y luego los demás programas.', 'Abre sempre primeiro a Aurora e depois os outros programas.'],
        ],
      },
      {
        id: 'log4om',
        title: ['Log4OM', 'Log4OM', 'Log4OM', 'Log4OM', 'Log4OM', 'Log4OM'],
        steps: [
          ['In Log4OM: Settings → Program configuration → CAT interface. CAT Engine: Hamlib.', 'In Log4OM: Settings → Program configuration → CAT interface. CAT Engine: Hamlib.', 'In Log4OM: Settings → Program configuration → CAT interface. CAT Engine: Hamlib.', 'Dans Log4OM : Settings → Program configuration → CAT interface. CAT Engine : Hamlib.', 'En Log4OM: Settings → Program configuration → CAT interface. CAT Engine: Hamlib.', 'No Log4OM: Settings → Program configuration → CAT interface. CAT Engine: Hamlib.'],
          ['Nella scheda Hamlib spunta «Connect to active HAMLIB instance» (127.0.0.1, 4532) e «VFO MODE» per vedere i due VFO.', 'In the Hamlib tab tick “Connect to active HAMLIB instance” (127.0.0.1, 4532) and “VFO MODE” to see both VFOs.', 'Im Reiter Hamlib „Connect to active HAMLIB instance“ (127.0.0.1, 4532) und „VFO MODE“ für beide VFOs anhaken.', 'Dans l’onglet Hamlib, coche « Connect to active HAMLIB instance » (127.0.0.1, 4532) et « VFO MODE » pour voir les deux VFO.', 'En la pestaña Hamlib marca «Connect to active HAMLIB instance» (127.0.0.1, 4532) y «VFO MODE» para ver los dos VFO.', 'No separador Hamlib marca «Connect to active HAMLIB instance» (127.0.0.1, 4532) e «VFO MODE» para ver os dois VFO.'],
          ['Se Log4OM mostra OFFLINE dopo un aggiornamento di Aurora, premi «Save and apply» per ricollegarlo.', 'If Log4OM shows OFFLINE after an Aurora update, press “Save and apply” to reconnect it.', 'Zeigt Log4OM nach einem Aurora-Update OFFLINE, „Save and apply“ drücken, um neu zu verbinden.', 'Si Log4OM affiche OFFLINE après une mise à jour d’Aurora, appuie sur « Save and apply » pour le reconnecter.', 'Si Log4OM muestra OFFLINE tras una actualización de Aurora, pulsa «Save and apply» para reconectarlo.', 'Se o Log4OM mostrar OFFLINE depois de uma atualização da Aurora, carrega em «Save and apply» para voltar a ligar.'],
        ],
      },
    ],
  },
  {
    id: 'pages',
    title: ['Le altre pagine', 'The other pages', 'Die anderen Seiten', 'Les autres pages', 'Las otras páginas', 'As outras páginas'],
    sections: [
      {
        id: 'map',
        title: ['[[nav.map]]', '[[nav.map]]', '[[nav.map]]', '[[nav.map]]', '[[nav.map]]', '[[nav.map]]'],
        text: ['Le stazioni che ricevi adesso e i quadratini già lavorati. Clic su una stazione per vederne la linea; rotella per ingrandire.', 'The stations you are receiving now and the squares already worked. Click a station to see its line; scroll to zoom.', 'Die Stationen, die du gerade empfängst, und die schon gearbeiteten Felder. Klick auf eine Station zeigt die Linie; Mausrad zoomt.', 'Les stations reçues maintenant et les carrés déjà contactés. Clique sur une station pour voir sa ligne ; molette pour zoomer.', 'Las estaciones que recibes ahora y los cuadrados ya trabajados. Clic en una estación para ver su línea; rueda para ampliar.', 'As estações que recebes agora e os quadrados já contactados. Clica numa estação para ver a linha; roda para ampliar.'],
      },
      {
        id: 'propagation',
        title: ['[[nav.propagation]]', '[[nav.propagation]]', '[[nav.propagation]]', '[[nav.propagation]]', '[[nav.propagation]]', '[[nav.propagation]]'],
        text: ['A sinistra sole e condizioni per banda; a destra la mappa e l’elenco di chi ti ha ricevuto (PSK Reporter). Si accende in [[nav.settings]] → [[tab.online]].', 'On the left the sun and band conditions; on the right the map and the list of who received you (PSK Reporter). Turn it on in [[nav.settings]] → [[tab.online]].', 'Links Sonne und Bandbedingungen; rechts Karte und Liste, wer dich empfangen hat (PSK Reporter). Einschalten in [[nav.settings]] → [[tab.online]].', 'À gauche le soleil et les conditions par bande ; à droite la carte et la liste de qui t’a reçu (PSK Reporter). À activer dans [[nav.settings]] → [[tab.online]].', 'A la izquierda el sol y las condiciones por banda; a la derecha el mapa y la lista de quién te ha recibido (PSK Reporter). Se activa en [[nav.settings]] → [[tab.online]].', 'À esquerda o sol e as condições por banda; à direita o mapa e a lista de quem te recebeu (PSK Reporter). Liga-se em [[nav.settings]] → [[tab.online]].'],
      },
      {
        id: 'stats',
        title: ['[[nav.stats]]', '[[nav.stats]]', '[[nav.stats]]', '[[nav.stats]]', '[[nav.stats]]', '[[nav.stats]]'],
        text: ['In alto i numeri del log (QSO, paesi, zone); sotto i QSO per banda e la scheda DXCC della stazione che stai lavorando. I paesi mancanti e il log ADIF si aprono con un clic.', 'At the top the log numbers (QSOs, countries, zones); below the QSOs per band and the DXCC card of the station you are working. Missing countries and the ADIF log open with a click.', 'Oben die Zahlen des Logs (QSOs, Länder, Zonen); darunter QSOs je Band und die DXCC-Karte der aktuellen Station. Fehlende Länder und das ADIF-Log öffnen sich per Klick.', 'En haut les chiffres du log (QSO, pays, zones) ; dessous les QSO par bande et la fiche DXCC de la station en cours. Les pays manquants et le log ADIF s’ouvrent d’un clic.', 'Arriba los números del log (QSO, países, zonas); debajo los QSO por banda y la ficha DXCC de la estación actual. Los países que faltan y el log ADIF se abren con un clic.', 'Em cima os números do log (QSO, países, zonas); por baixo os QSO por banda e a ficha DXCC da estação atual. Os países em falta e o log ADIF abrem-se com um clique.'],
      },
      {
        id: 'history',
        title: ['[[nav.history]]', '[[nav.history]]', '[[nav.history]]', '[[nav.history]]', '[[nav.history]]', '[[nav.history]]'],
        text: ['Tutti i messaggi ricevuti negli ultimi giorni, da cercare per nominativo, banda o paese.', 'All messages received in recent days, searchable by callsign, band or country.', 'Alle Nachrichten der letzten Tage, durchsuchbar nach Rufzeichen, Band oder Land.', 'Tous les messages reçus ces derniers jours, à rechercher par indicatif, bande ou pays.', 'Todos los mensajes recibidos en los últimos días, para buscar por indicativo, banda o país.', 'Todas as mensagens recebidas nos últimos dias, para pesquisar por indicativo, banda ou país.'],
      },
    ],
  },
  {
    id: 'alerts',
    title: ['Avvisi e orologio', 'Alerts and clock', 'Hinweise und Uhr', 'Alertes et horloge', 'Avisos y reloj', 'Avisos e relógio'],
    sections: [
      {
        id: 'sounds',
        title: ['Suoni', 'Sounds', 'Töne', 'Sons', 'Sonidos', 'Sons'],
        steps: [
          ['In [[nav.settings]] → [[tab.notifications]] scegli un suono per ogni evento.', 'In [[nav.settings]] → [[tab.notifications]] pick a sound for each event.', 'In [[nav.settings]] → [[tab.notifications]] für jedes Ereignis einen Ton wählen.', 'Dans [[nav.settings]] → [[tab.notifications]], choisis un son pour chaque événement.', 'En [[nav.settings]] → [[tab.notifications]] elige un sonido para cada evento.', 'Em [[nav.settings]] → [[tab.notifications]] escolhe um som para cada evento.'],
          ['Il più utile: «chiamata verso di me». Suona ogni volta che una stazione ti risponde.', 'The most useful: “call to me”. It rings every time a station answers you.', 'Am nützlichsten: „Anruf an mich“. Ertönt jedes Mal, wenn eine Station dir antwortet.', 'Le plus utile : « appel vers moi ». Il sonne chaque fois qu’une station te répond.', 'El más útil: «llamada hacia mí». Suena cada vez que una estación te responde.', 'O mais útil: «chamada para mim». Toca sempre que uma estação te responde.'],
          ['Le stazioni che cerchi: aggiungile in [[nav.settings]] → [[tab.wishlist]].', 'Stations you are looking for: add them in [[nav.settings]] → [[tab.wishlist]].', 'Gesuchte Stationen: in [[nav.settings]] → [[tab.wishlist]] eintragen.', 'Les stations recherchées : ajoute-les dans [[nav.settings]] → [[tab.wishlist]].', 'Las estaciones que buscas: añádelas en [[nav.settings]] → [[tab.wishlist]].', 'As estações que procuras: adiciona-as em [[nav.settings]] → [[tab.wishlist]].'],
        ],
      },
      {
        id: 'clock',
        title: ['L’orologio', 'The clock', 'Die Uhr', 'L’horloge', 'El reloj', 'O relógio'],
        text: ['In FT8 l’ora del PC deve essere giusta entro una frazione di secondo. Se va tutto bene non vedi niente; se no compare un avviso giallo in alto.', 'In FT8 the PC time must be right within a fraction of a second. If all is well you see nothing; otherwise a yellow warning appears at the top.', 'Bei FT8 muss die PC-Zeit auf Bruchteile einer Sekunde stimmen. Ist alles gut, siehst du nichts; sonst erscheint oben eine gelbe Warnung.', 'En FT8 l’heure du PC doit être juste à une fraction de seconde près. Si tout va bien tu ne vois rien ; sinon un avertissement jaune apparaît en haut.', 'En FT8 la hora del PC debe ser exacta con una fracción de segundo. Si todo va bien no ves nada; si no, aparece un aviso amarillo arriba.', 'Em FT8 a hora do PC tem de estar certa a uma fração de segundo. Se estiver tudo bem não vês nada; senão aparece um aviso amarelo em cima.'],
        steps: [
          ['Clicca l’avviso: si apre [[nav.settings]] → [[tab.clock]], con la prova sui server NTP.', 'Click the warning: [[nav.settings]] → [[tab.clock]] opens, with the NTP server test.', 'Klicke die Warnung: [[nav.settings]] → [[tab.clock]] öffnet sich mit dem NTP-Test.', 'Clique l’avertissement : [[nav.settings]] → [[tab.clock]] s’ouvre, avec le test NTP.', 'Pulsa el aviso: se abre [[nav.settings]] → [[tab.clock]], con la prueba NTP.', 'Clica no aviso: abre [[nav.settings]] → [[tab.clock]], com o teste NTP.'],
          ['Accendi «[[clock.atStart]]»: a ogni avvio Aurora corregge l’ora da sola. Per cambiare l’ora del PC Aurora deve partire come amministratore.', 'Turn on “[[clock.atStart]]”: at every start Aurora corrects the time by itself. To change the PC time Aurora must run as administrator.', '„[[clock.atStart]]“ einschalten: bei jedem Start korrigiert Aurora die Zeit selbst. Dafür muss Aurora als Administrator laufen.', 'Active « [[clock.atStart]] » : à chaque démarrage Aurora corrige l’heure seule. Pour cela Aurora doit être lancée en administrateur.', 'Activa «[[clock.atStart]]»: en cada inicio Aurora corrige la hora sola. Para ello Aurora debe ejecutarse como administrador.', 'Liga «[[clock.atStart]]»: em cada arranque a Aurora acerta a hora sozinha. Para isso a Aurora tem de correr como administrador.'],
        ],
      },
    ],
  },
  {
    id: 'phone',
    title: ['Telefono e aggiornamenti', 'Phone and updates', 'Handy und Updates', 'Téléphone et mises à jour', 'Teléfono y actualizaciones', 'Telemóvel e atualizações'],
    sections: [
      {
        id: 'phone',
        title: ['Usare Aurora dal telefono', 'Using Aurora from the phone', 'Aurora am Handy nutzen', 'Utiliser Aurora depuis le téléphone', 'Usar Aurora desde el teléfono', 'Usar a Aurora no telemóvel'],
        steps: [
          ['Sul PC apri [[nav.settings]] → [[tab.access]] e crea un codice di associazione.', 'On the PC open [[nav.settings]] → [[tab.access]] and create a pairing code.', 'Am PC [[nav.settings]] → [[tab.access]] öffnen und einen Kopplungscode erzeugen.', 'Sur le PC ouvre [[nav.settings]] → [[tab.access]] et crée un code d’association.', 'En el PC abre [[nav.settings]] → [[tab.access]] y crea un código de emparejamiento.', 'No PC abre [[nav.settings]] → [[tab.access]] e cria um código de associação.'],
          ['Sul telefono apri l’indirizzo mostrato e scrivi il codice. Il telefono resta associato finché non lo togli.', 'On the phone open the address shown and enter the code. The phone stays paired until you remove it.', 'Am Handy die angezeigte Adresse öffnen und den Code eingeben. Das Handy bleibt gekoppelt, bis du es entfernst.', 'Sur le téléphone ouvre l’adresse affichée et saisis le code. Le téléphone reste associé jusqu’à ce que tu le retires.', 'En el teléfono abre la dirección mostrada y escribe el código. El teléfono queda emparejado hasta que lo quites.', 'No telemóvel abre o endereço mostrado e escreve o código. O telemóvel fica associado até o removeres.'],
        ],
      },
      {
        id: 'updates',
        title: ['Aggiornamenti', 'Updates', 'Updates', 'Mises à jour', 'Actualizaciones', 'Atualizações'],
        text: ['Aurora controlla da sola se c’è una versione nuova e la propone. L’installatore chiude Aurora e la riapre. Dopo un aggiornamento ricollega i programmi che usano la radio tramite Aurora.', 'Aurora checks for new versions by itself and offers them. The installer closes Aurora and reopens it. After an update reconnect the programs that use the radio through Aurora.', 'Aurora prüft selbst auf neue Versionen und bietet sie an. Der Installer schließt und öffnet Aurora. Nach einem Update die Programme neu verbinden, die das Funkgerät über Aurora nutzen.', 'Aurora vérifie seule les nouvelles versions et les propose. L’installateur ferme Aurora et la rouvre. Après une mise à jour, reconnecte les programmes qui utilisent la radio via Aurora.', 'Aurora comprueba sola si hay una versión nueva y la ofrece. El instalador cierra Aurora y la vuelve a abrir. Tras una actualización reconecta los programas que usan la radio a través de Aurora.', 'A Aurora verifica sozinha se há uma versão nova e propõe-na. O instalador fecha a Aurora e volta a abri-la. Depois de uma atualização volta a ligar os programas que usam o rádio através da Aurora.'],
      },
    ],
  },
];
