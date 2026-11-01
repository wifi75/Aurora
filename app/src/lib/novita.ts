import type { Entry } from '../i18n/core';

/**
 * LE ULTIME 20 NOVITA' di Aurora, dalla piu' recente. REGOLA: a ogni versione (nuova funzione, modifica o correzione) si aggiunge qui la voce
 * in TUTTE e sei le lingue [it, en, de, fr, es, pt], con la data e l'ora UTC del rilascio, e si toglie la ventunesima. Una prova controlla che la
 * voce piu' recente sia della versione in uso.
 */
export type NovitaKind = 'new' | 'changed' | 'fixed';

export interface Novita {
  version: string;
  /** Data e ora del rilascio su GitHub, in UTC (si mostra nell'ora del dispositivo). */
  released: string;
  kind: NovitaKind;
  title: Entry;
  text: Entry;
}

export const NOVITA_MAX = 20;

export const NOVITA: readonly Novita[] = [
  {
    version: '0.11.48',
    released: '2026-11-01T07:56:00Z',
    kind: 'new',
    title: ['Pagina Novità', 'News page', 'Seite „Neuigkeiten“', 'Page Nouveautés', 'Página de novedades', 'Página de novidades'],
    text: [
      'Dopo la Guida c’è il pulsante Novità: le ultime 20 funzioni aggiunte, modificate o corrette, con data e ora del rilascio.',
      'After the Guide there is a News button: the latest 20 features added, changed or fixed, with the release date and time.',
      'Nach der Anleitung gibt es die Schaltfläche Neuigkeiten: die letzten 20 neuen, geänderten oder korrigierten Funktionen mit Datum und Uhrzeit der Veröffentlichung.',
      'Après le Guide, un bouton Nouveautés : les 20 dernières fonctions ajoutées, modifiées ou corrigées, avec la date et l’heure de publication.',
      'Tras la Guía hay un botón Novedades: las últimas 20 funciones añadidas, modificadas o corregidas, con la fecha y hora de publicación.',
      'Depois do Guia há o botão Novidades: as últimas 20 funções adicionadas, alteradas ou corrigidas, com a data e a hora da publicação.',
    ],
  },
  {
    version: '0.11.48',
    released: '2026-11-01T07:56:00Z',
    kind: 'new',
    title: ['Messaggi toast personalizzabili e immediati', 'Customisable, instant toast messages', 'Anpassbare, sofortige Toast-Meldungen', 'Messages toast personnalisables et immédiats', 'Mensajes toast personalizables e inmediatos', 'Mensagens toast personalizáveis e imediatas'],
    text: [
      'In Impostazioni > Notifiche scegli quali avvisi mostrare, per quanti secondi, dove (in alto, a destra, in basso) e di che dimensione. L’avviso compare subito al clic.',
      'In Settings > Notifications choose which notices to show, for how long, where (top, right, bottom) and how big. The notice appears at once on click.',
      'Unter Einstellungen > Benachrichtigungen wählst du, welche Hinweise, wie lange, wo (oben, rechts, unten) und wie groß. Der Hinweis erscheint sofort beim Klick.',
      'Dans Réglages > Notifications, choisissez quels avis, combien de secondes, où (haut, droite, bas) et quelle taille. L’avis apparaît dès le clic.',
      'En Ajustes > Notificaciones eliges qué avisos, cuántos segundos, dónde (arriba, derecha, abajo) y de qué tamaño. El aviso aparece al instante con el clic.',
      'Em Definições > Notificações escolhes que avisos, quantos segundos, onde (cima, direita, baixo) e que tamanho. O aviso aparece logo ao clicar.',
    ],
  },
  {
    version: '0.11.48',
    released: '2026-11-01T07:56:00Z',
    kind: 'changed',
    title: ['CQ e novità a metà, DXpedition a colori', 'CQ and new ones split in half, coloured DXpeditions', 'CQ und Neues halbiert, farbige DXpeditionen', 'CQ et nouveautés en deux moitiés, DXpéditions en couleur', 'CQ y novedades a la mitad, DXpediciones a color', 'CQ e novidades a meio, DXpedições a cores'],
    text: [
      'Il pannello è diviso esattamente a metà: CQ a sinistra, DXpedition a destra con il titolo nella barra. Le schede hanno colori per stato, bandiera e data.',
      'The panel is split exactly in half: CQ on the left, DXpeditions on the right with the title in the bar. Cards are coloured by state, with flag and date.',
      'Das Fenster ist genau halbiert: links CQ, rechts DXpeditionen mit dem Titel in der Leiste. Die Karten sind nach Status gefärbt, mit Flagge und Datum.',
      'Le panneau est coupé exactement en deux : CQ à gauche, DXpéditions à droite avec le titre dans la barre. Les fiches sont colorées selon l’état, avec drapeau et date.',
      'El panel se divide exactamente por la mitad: CQ a la izquierda, DXpediciones a la derecha con el título en la barra. Las fichas tienen color según el estado, bandera y fecha.',
      'O painel divide-se exatamente a meio: CQ à esquerda, DXpedições à direita com o título na barra. As fichas têm cor por estado, bandeira e data.',
    ],
  },
  {
    version: '0.11.48',
    released: '2026-11-01T07:56:00Z',
    kind: 'fixed',
    title: ['Doppio clic: parte la trasmissione', 'Double-click: transmission starts', 'Doppelklick: die Sendung startet', 'Double-clic : l’émission démarre', 'Doble clic: arranca la transmisión', 'Duplo clique: a transmissão arranca'],
    text: [
      'Il doppio clic su una stazione ora sceglie prima la risposta e subito dopo accende Abilita Tx; prima le due richieste partivano insieme e Abilita Tx poteva non accendersi.',
      'Double-clicking a station now picks the reply first and then turns on Enable Tx right after; before, both requests went out together and Enable Tx could fail to turn on.',
      'Ein Doppelklick auf eine Station wählt jetzt zuerst die Antwort und schaltet danach „Tx aktivieren“ ein; vorher liefen beide Anfragen gleichzeitig und Tx wurde manchmal nicht aktiviert.',
      'Le double-clic sur une station choisit d’abord la réponse puis active Autoriser Tx juste après ; avant, les deux demandes partaient ensemble et Autoriser Tx pouvait ne pas s’activer.',
      'El doble clic en una estación ahora elige primero la respuesta y enseguida activa Habilitar Tx; antes las dos peticiones salían juntas y Habilitar Tx podía no encenderse.',
      'O duplo clique numa estação agora escolhe primeiro a resposta e logo a seguir liga Ativar Tx; antes os dois pedidos saíam juntos e o Ativar Tx podia não ligar.',
    ],
  },
  {
    version: '0.11.48',
    released: '2026-11-01T07:56:00Z',
    kind: 'changed',
    title: ['Impostazioni più compatte e ordinate', 'More compact, tidier settings', 'Kompaktere, übersichtlichere Einstellungen', 'Réglages plus compacts et ordonnés', 'Ajustes más compactos y ordenados', 'Definições mais compactas e organizadas'],
    text: [
      'La casella di spunta sta accanto al suo testo, le tabelle corte sono strette, le macro in griglia, lingua e tema in un riquadro; il Collegamento radio ha caselle larghe quanto serve.',
      'The checkbox sits next to its text, short tables are narrow, macros are in a grid, language and theme share one box; the radio Connection has fields as wide as needed.',
      'Das Kontrollkästchen steht neben seinem Text, kurze Tabellen sind schmal, Makros im Raster, Sprache und Design in einem Feld; die Funkverbindung hat Felder in passender Breite.',
      'La case à cocher est à côté de son texte, les tableaux courts sont étroits, les macros en grille, langue et thème dans un seul cadre ; la Connexion radio a des champs de la bonne largeur.',
      'La casilla está junto a su texto, las tablas cortas son estrechas, las macros en cuadrícula, idioma y tema en un recuadro; la Conexión de radio tiene campos del ancho necesario.',
      'A caixa de seleção fica junto ao texto, as tabelas curtas são estreitas, as macros em grelha, idioma e tema numa caixa; a Ligação do rádio tem campos com a largura necessária.',
    ],
  },
  {
    version: '0.11.48',
    released: '2026-11-01T07:56:00Z',
    kind: 'changed',
    title: ['RX/TX più piccola, un clic su TX ferma', 'Smaller RX/TX, one click on TX stops', 'Kleineres RX/TX, ein Klick auf TX stoppt', 'RX/TX plus petit, un clic sur TX arrête', 'RX/TX más pequeño, un clic en TX detiene', 'RX/TX mais pequeno, um clique em TX para'],
    text: [
      'La pastiglia TX ha la stessa misura e lo stesso posto di RX, senza la scritta STOP; un clic su TX ferma la trasmissione.',
      'The TX pill has the same size and place as RX, without the STOP label; a click on TX stops the transmission.',
      'Die TX-Anzeige hat dieselbe Größe und Stelle wie RX, ohne den Text STOP; ein Klick auf TX stoppt die Sendung.',
      'La pastille TX a la même taille et la même place que RX, sans le mot STOP ; un clic sur TX arrête l’émission.',
      'La pastilla TX tiene el mismo tamaño y lugar que RX, sin el texto STOP; un clic en TX detiene la transmisión.',
      'A pastilha TX tem o mesmo tamanho e lugar que a RX, sem o texto STOP; um clique em TX para a transmissão.',
    ],
  },
  {
    version: '0.11.47',
    released: '2026-11-01T07:42:35Z',
    kind: 'changed',
    title: ['Tutta la radio nella scheda Radio', 'All radio settings in the Radio tab', 'Alles zum Funkgerät im Reiter Radio', 'Toute la radio dans l’onglet Radio', 'Toda la radio en la pestaña Radio', 'Todo o rádio no separador Rádio'],
    text: [
      'Dal CAT al display esteso, tutto sta in Impostazioni > Radio. La testata della Stazione è compatta e il display esteso ha un’area fissa.',
      'From CAT to the extended display, everything is in Settings > Radio. The Station header is compact and the extended display has a fixed area.',
      'Von CAT bis zur erweiterten Anzeige ist alles unter Einstellungen > Radio. Die Kopfzeile der Station ist kompakt, die erweiterte Anzeige hat einen festen Bereich.',
      'Du CAT à l’affichage étendu, tout est dans Réglages > Radio. L’en-tête de la Station est compact et l’affichage étendu a une zone fixe.',
      'Del CAT al display ampliado, todo está en Ajustes > Radio. La cabecera de la Estación es compacta y el display ampliado tiene un área fija.',
      'Do CAT ao visor alargado, tudo está em Definições > Rádio. O cabeçalho da Estação é compacto e o visor alargado tem uma área fixa.',
    ],
  },
  {
    version: '0.11.47',
    released: '2026-11-01T07:42:35Z',
    kind: 'fixed',
    title: ['Spiegazioni dei «?» di nuovo visibili', 'The “?” explanations show again', 'Die „?“-Erklärungen sind wieder sichtbar', 'Les explications des « ? » s’affichent à nouveau', 'Las explicaciones de los «?» se ven de nuevo', 'As explicações dos «?» voltam a ver-se'],
    text: [
      'La nuvoletta restava tagliata dentro l’etichetta: ora compare sopra a tutto passando il mouse sul «?», e un clic la tiene aperta.',
      'The bubble was clipped inside the label: it now appears above everything when you hover the “?”, and a click keeps it open.',
      'Die Sprechblase wurde im Etikett abgeschnitten: Jetzt erscheint sie über allem, wenn du über das „?“ fährst; ein Klick hält sie offen.',
      'La bulle était coupée dans l’étiquette : elle apparaît maintenant au-dessus de tout en survolant le « ? », et un clic la garde ouverte.',
      'El globo quedaba cortado dentro de la etiqueta: ahora aparece sobre todo al pasar el ratón por el «?», y un clic lo mantiene abierto.',
      'O balão ficava cortado dentro da etiqueta: agora aparece por cima de tudo ao passar o rato no «?», e um clique mantém-no aberto.',
    ],
  },
  {
    version: '0.11.46',
    released: '2026-11-01T07:26:26Z',
    kind: 'new',
    title: ['Display esteso della radio', 'Extended radio display', 'Erweiterte Funkgeräteanzeige', 'Affichage étendu de la radio', 'Display ampliado de la radio', 'Visor alargado do rádio'],
    text: [
      'Sotto il display: secondo VFO, strumenti, PRE/ATT, AGC, NB/NR/notch, guadagni, RIT/XIT, ATU. Ogni voce si accende in Impostazioni > Radio, con la spiegazione.',
      'Under the display: second VFO, meters, PRE/ATT, AGC, NB/NR/notch, gains, RIT/XIT, ATU. Each item is switched on in Settings > Radio, with its explanation.',
      'Unter der Anzeige: zweiter VFO, Messgeräte, PRE/ATT, AGC, NB/NR/Notch, Pegel, RIT/XIT, ATU. Jeder Punkt wird unter Einstellungen > Radio eingeschaltet, mit Erklärung.',
      'Sous l’affichage : second VFO, instruments, PRE/ATT, AGC, NB/NR/notch, gains, RIT/XIT, ATU. Chaque élément s’active dans Réglages > Radio, avec son explication.',
      'Bajo el display: segundo VFO, instrumentos, PRE/ATT, AGC, NB/NR/notch, ganancias, RIT/XIT, ATU. Cada elemento se enciende en Ajustes > Radio, con su explicación.',
      'Sob o visor: segundo VFO, instrumentos, PRE/ATT, AGC, NB/NR/notch, ganhos, RIT/XIT, ATU. Cada item liga-se em Definições > Rádio, com a explicação.',
    ],
  },
  {
    version: '0.11.45',
    released: '2026-11-01T07:19:53Z',
    kind: 'new',
    title: ['DXpedition dal calendario NG3K', 'DXpeditions from the NG3K calendar', 'DXpeditionen aus dem NG3K-Kalender', 'DXpéditions du calendrier NG3K', 'DXpediciones del calendario NG3K', 'DXpedições do calendário NG3K'],
    text: [
      'Il pannello «CQ e novità» mostra le DXpedition in onda, quelle sentite adesso, quelle in arrivo e lo storico. Si accende in Impostazioni > Servizi online.',
      'The “CQ and new ones” panel shows DXpeditions on air, heard right now, coming up, and the history. Switch it on in Settings > Online services.',
      'Das Fenster „CQ und Neues“ zeigt DXpeditionen auf Sendung, gerade gehörte, kommende und den Verlauf. Einschalten unter Einstellungen > Online-Dienste.',
      'Le panneau « CQ et nouveautés » montre les DXpéditions en ondes, entendues maintenant, à venir et l’historique. À activer dans Réglages > Services en ligne.',
      'El panel «CQ y novedades» muestra las DXpediciones en el aire, oídas ahora, próximas y el historial. Se activa en Ajustes > Servicios en línea.',
      'O painel «CQ e novidades» mostra as DXpedições no ar, ouvidas agora, próximas e o histórico. Liga-se em Definições > Serviços online.',
    ],
  },
  {
    version: '0.11.45',
    released: '2026-11-01T07:19:53Z',
    kind: 'changed',
    title: ['STOP nella pastiglia TX', 'STOP in the TX pill', 'STOP in der TX-Anzeige', 'STOP dans la pastille TX', 'STOP en la pastilla TX', 'STOP na pastilha TX'],
    text: [
      'Niente pulsante STOP a parte: in onda la pastiglia rossa TX è anche lo STOP.',
      'No separate STOP button: while on air the red TX pill is also the STOP.',
      'Kein eigener STOP-Knopf: Auf Sendung ist die rote TX-Anzeige zugleich der STOP.',
      'Pas de bouton STOP séparé : en émission, la pastille rouge TX est aussi le STOP.',
      'Sin botón STOP aparte: en el aire, la pastilla roja TX es también el STOP.',
      'Sem botão STOP à parte: no ar, a pastilha vermelha TX é também o STOP.',
    ],
  },
  {
    version: '0.11.44',
    released: '2026-11-01T07:06:21Z',
    kind: 'new',
    title: ['Km e azimut anche senza locatore', 'Km and bearing even without a locator', 'Km und Azimut auch ohne Locator', 'Km et azimut même sans locator', 'Km y azimut aun sin locator', 'Km e azimute mesmo sem locator'],
    text: [
      'Sui messaggi senza locatore la distanza e l’azimut si calcolano col locatore già mandato da quella stazione.',
      'On messages without a locator, distance and bearing use the locator that station already sent.',
      'Bei Nachrichten ohne Locator werden Entfernung und Azimut mit dem zuvor von dieser Station gesendeten Locator berechnet.',
      'Sur les messages sans locator, la distance et l’azimut utilisent le locator déjà envoyé par cette station.',
      'En los mensajes sin locator, la distancia y el azimut usan el locator que esa estación ya envió.',
      'Nas mensagens sem locator, a distância e o azimute usam o locator que essa estação já enviou.',
    ],
  },
  {
    version: '0.11.43',
    released: '2026-11-01T06:58:16Z',
    kind: 'new',
    title: ['Cascata fedele al filtro della radio', 'Waterfall that follows the radio filter', 'Wasserfall folgt dem Funkgerätefilter', 'Cascade fidèle au filtre de la radio', 'Cascada fiel al filtro de la radio', 'Cascata fiel ao filtro do rádio'],
    text: [
      'Aurora legge via CAT la larghezza del filtro della radio e la cascata mostra quell’ampiezza, fino a 5000 Hz.',
      'Aurora reads the radio filter width over CAT and the waterfall shows that span, up to 5000 Hz.',
      'Aurora liest die Filterbreite des Funkgeräts über CAT, und der Wasserfall zeigt diese Breite, bis 5000 Hz.',
      'Aurora lit par CAT la largeur du filtre de la radio et la cascade montre cette largeur, jusqu’à 5000 Hz.',
      'Aurora lee por CAT el ancho del filtro de la radio y la cascada muestra ese ancho, hasta 5000 Hz.',
      'O Aurora lê por CAT a largura do filtro do rádio e a cascata mostra essa largura, até 5000 Hz.',
    ],
  },
  {
    version: '0.11.43',
    released: '2026-11-01T06:58:16Z',
    kind: 'new',
    title: ['Coda sempre visibile', 'Queue always visible', 'Warteschlange immer sichtbar', 'File d’attente toujours visible', 'Cola siempre visible', 'Fila sempre visível'],
    text: [
      'Nel pannello Trasmissione, sotto il DX, «In coda» mostra con pallini blu numerati i nominativi che aspettano.',
      'In the Transmission panel, under the DX, “Queued” shows the waiting callsigns with numbered blue dots.',
      'Im Sendefenster zeigt unter dem DX „In Warteschlange“ die wartenden Rufzeichen mit nummerierten blauen Punkten.',
      'Dans le panneau Émission, sous le DX, « En file » montre les indicatifs en attente avec des pastilles bleues numérotées.',
      'En el panel Transmisión, bajo el DX, «En cola» muestra con puntos azules numerados los indicativos que esperan.',
      'No painel Transmissão, sob o DX, «Em fila» mostra com pontos azuis numerados os indicativos à espera.',
    ],
  },
  {
    version: '0.11.41',
    released: '2026-11-01T06:15:58Z',
    kind: 'changed',
    title: ['Impostazioni riordinate con ricerca e «?»', 'Settings reorganised with search and “?”', 'Einstellungen neu geordnet mit Suche und „?“', 'Réglages réorganisés avec recherche et « ? »', 'Ajustes reorganizados con búsqueda y «?»', 'Definições reorganizadas com pesquisa e «?»'],
    text: [
      'Barra laterale con ricerca, pagine compatte e un «?» con la spiegazione di ogni voce, in sei lingue.',
      'Sidebar with search, compact pages and a “?” explaining every item, in six languages.',
      'Seitenleiste mit Suche, kompakte Seiten und ein „?“ mit Erklärung zu jedem Eintrag, in sechs Sprachen.',
      'Barre latérale avec recherche, pages compactes et un « ? » expliquant chaque élément, en six langues.',
      'Barra lateral con búsqueda, páginas compactas y un «?» que explica cada elemento, en seis idiomas.',
      'Barra lateral com pesquisa, páginas compactas e um «?» que explica cada item, em seis línguas.',
    ],
  },
  {
    version: '0.11.40',
    released: '2026-10-31T17:43:10Z',
    kind: 'new',
    title: ['Guida riscritta', 'Guide rewritten', 'Anleitung neu geschrieben', 'Guide réécrit', 'Guía reescrita', 'Guia reescrito'],
    text: [
      'Otto capitoli con indice, ricerca e passi numerati, in sei lingue; i nomi di schede e pulsanti vengono dall’interfaccia.',
      'Eight chapters with index, search and numbered steps, in six languages; tab and button names come from the interface.',
      'Acht Kapitel mit Inhaltsverzeichnis, Suche und nummerierten Schritten, in sechs Sprachen; die Namen von Reitern und Schaltflächen stammen aus der Oberfläche.',
      'Huit chapitres avec sommaire, recherche et étapes numérotées, en six langues ; les noms d’onglets et de boutons viennent de l’interface.',
      'Ocho capítulos con índice, búsqueda y pasos numerados, en seis idiomas; los nombres de pestañas y botones vienen de la interfaz.',
      'Oito capítulos com índice, pesquisa e passos numerados, em seis línguas; os nomes de separadores e botões vêm da interface.',
    ],
  },
  {
    version: '0.11.38',
    released: '2026-10-31T16:42:47Z',
    kind: 'fixed',
    title: ['Radio condivisa con Log4OM: due VFO', 'Radio shared with Log4OM: two VFOs', 'Funkgerät mit Log4OM geteilt: zwei VFOs', 'Radio partagée avec Log4OM : deux VFO', 'Radio compartida con Log4OM: dos VFO', 'Rádio partilhado com o Log4OM: dois VFO'],
    text: [
      'Con la condivisione accesa Aurora usa i due VFO come chiede Log4OM («VFO MODE»); prima Log4OM perdeva il primo VFO.',
      'With sharing on, Aurora uses the two VFOs the way Log4OM asks (“VFO MODE”); before, Log4OM lost the first VFO.',
      'Bei eingeschaltetem Teilen nutzt Aurora die zwei VFOs wie von Log4OM verlangt („VFO MODE“); vorher verlor Log4OM den ersten VFO.',
      'Avec le partage activé, Aurora utilise les deux VFO comme le demande Log4OM (« VFO MODE ») ; avant, Log4OM perdait le premier VFO.',
      'Con el uso compartido activo, Aurora usa los dos VFO como pide Log4OM («VFO MODE»); antes Log4OM perdía el primer VFO.',
      'Com a partilha ligada, o Aurora usa os dois VFO como o Log4OM pede («VFO MODE»); antes o Log4OM perdia o primeiro VFO.',
    ],
  },
  {
    version: '0.11.35',
    released: '2026-10-31T16:22:07Z',
    kind: 'changed',
    title: ['Pwr = volume di Windows verso la radio', 'Pwr = Windows volume towards the radio', 'Pwr = Windows-Lautstärke zum Funkgerät', 'Pwr = volume Windows vers la radio', 'Pwr = volumen de Windows hacia la radio', 'Pwr = volume do Windows para o rádio'],
    text: [
      'Il cursore Pwr regola il volume della scheda audio verso la radio; il segnale generato resta pieno, e S-meter e potenza non spariscono più per un errore momentaneo.',
      'The Pwr slider sets the sound-card volume towards the radio; the generated signal stays full, and the S-meter and power no longer vanish after a momentary error.',
      'Der Pwr-Regler stellt die Lautstärke der Soundkarte zum Funkgerät ein; das erzeugte Signal bleibt voll, S-Meter und Leistung verschwinden nicht mehr bei einem kurzen Fehler.',
      'Le curseur Pwr règle le volume de la carte son vers la radio ; le signal généré reste plein, et le S-mètre et la puissance ne disparaissent plus après une erreur passagère.',
      'El control Pwr ajusta el volumen de la tarjeta de sonido hacia la radio; la señal generada queda completa, y el S-meter y la potencia ya no desaparecen por un error momentáneo.',
      'O cursor Pwr regula o volume da placa de som para o rádio; o sinal gerado fica completo, e o S-meter e a potência já não desaparecem por um erro momentâneo.',
    ],
  },
  {
    version: '0.11.33',
    released: '2026-10-31T14:09:50Z',
    kind: 'new',
    title: ['AGCc: compensazione dell’AGC della radio', 'AGCc: compensating the radio AGC', 'AGCc: Ausgleich der AGC des Funkgeräts', 'AGCc : compensation de l’AGC de la radio', 'AGCc: compensación del AGC de la radio', 'AGCc: compensação do AGC do rádio'],
    text: [
      'Quando l’AGC cambia il guadagno (una stazione forte che parte o finisce), quei tratti tornano al livello giusto e si decodificano meglio; portato da JTDX e migliorato.',
      'When the AGC changes the gain (a strong station starting or stopping), those stretches return to the right level and decode better; ported from JTDX and improved.',
      'Wenn die AGC die Verstärkung ändert (eine starke Station beginnt oder endet), kehren diese Abschnitte auf den richtigen Pegel zurück und werden besser decodiert; von JTDX übernommen und verbessert.',
      'Quand l’AGC change le gain (une station forte qui démarre ou s’arrête), ces passages reviennent au bon niveau et se décodent mieux ; repris de JTDX et amélioré.',
      'Cuando el AGC cambia la ganancia (una estación fuerte que empieza o termina), esos tramos vuelven al nivel correcto y se decodifican mejor; portado de JTDX y mejorado.',
      'Quando o AGC muda o ganho (uma estação forte que começa ou acaba), esses trechos voltam ao nível certo e descodificam melhor; trazido do JTDX e melhorado.',
    ],
  },
  {
    version: '0.11.24',
    released: '2026-10-26T20:16:48Z',
    kind: 'new',
    title: ['JT65 e JT9 nel motore proprio', 'JT65 and JT9 in the own engine', 'JT65 und JT9 im eigenen Motor', 'JT65 et JT9 dans le moteur propre', 'JT65 y JT9 en el motor propio', 'JT65 e JT9 no motor próprio'],
    text: [
      'Nel selettore dei modi ci sono FT8, FT4, JT65 e JT9: decodifica del minuto intero con sottrazione dei segnali, e trasmissione.',
      'The mode selector now has FT8, FT4, JT65 and JT9: full-minute decoding with signal subtraction, and transmission.',
      'Der Modusschalter bietet FT8, FT4, JT65 und JT9: Dekodierung der ganzen Minute mit Signalabzug, und Senden.',
      'Le sélecteur de modes propose FT8, FT4, JT65 et JT9 : décodage de la minute entière avec soustraction des signaux, et émission.',
      'El selector de modos tiene FT8, FT4, JT65 y JT9: decodificación del minuto entero con sustracción de señales, y transmisión.',
      'O seletor de modos tem FT8, FT4, JT65 e JT9: descodificação do minuto inteiro com subtração de sinais, e transmissão.',
    ],
  },
];
