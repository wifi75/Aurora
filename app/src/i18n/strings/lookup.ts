import type { Entry } from '../core';

/** Ricerca dei nominativi (QRZ.com, HamQTH), riquadro del nominativo nel DX Cluster, HRDLOG.NET e campi segreti. [it, en, de, fr, es, pt] */
export const lookupStrings = {
  // riquadro al passaggio del mouse sul nominativo
  'calltip.loading': ['ricerca…', 'looking up…', 'suche…', 'recherche…', 'buscando…', 'a procurar…'],
  'calltip.unknown': ['paese non riconosciuto', 'country not recognised', 'Land nicht erkannt', 'pays non reconnu', 'país no reconocido', 'país não reconhecido'],
  'calltip.source': ['da {source}', 'from {source}', 'von {source}', 'depuis {source}', 'de {source}', 'de {source}'],
  'calltip.reduced': [
    'dati ridotti: senza abbonamento XML di QRZ',
    'reduced data: no QRZ XML subscription',
    'reduzierte Daten: kein QRZ-XML-Abo',
    'données réduites : pas d’abonnement XML QRZ',
    'datos reducidos: sin suscripción XML de QRZ',
    'dados reduzidos: sem assinatura XML do QRZ',
  ],

  // campi segreti
  'ens.pw.savedShort': ['Chiave salvata', 'Key saved', 'Schlüssel gespeichert', 'Clé enregistrée', 'Clave guardada', 'Chave guardada'],
  'ens.pw.change': ['Cambia', 'Change', 'Ändern', 'Changer', 'Cambiar', 'Mudar'],
  'ens.pw.changeTip': ['Scrivi una nuova password o chiave al posto di quella salvata.', 'Type a new password or key to replace the saved one.', 'Ein neues Passwort oder einen neuen Schlüssel statt des gespeicherten eingeben.', 'Écrire un nouveau mot de passe ou une nouvelle clé à la place de l’actuel.', 'Escribe una nueva contraseña o clave en lugar de la guardada.', 'Escreve uma nova palavra-passe ou chave no lugar da guardada.'],
  'ens.pw.remove': ['Togli', 'Remove', 'Entfernen', 'Retirer', 'Quitar', 'Remover'],
  'ens.pw.removeTip': ['Cancella da questo PC la password o la chiave salvata.', 'Delete the saved password or key from this PC.', 'Das gespeicherte Passwort bzw. den Schlüssel von diesem PC löschen.', 'Effacer de ce PC le mot de passe ou la clé enregistrés.', 'Borra de este PC la contraseña o clave guardada.', 'Apaga deste PC a palavra-passe ou chave guardada.'],
  'ens.pw.type': ['scrivi e premi Invio', 'type and press Enter', 'eingeben und Enter drücken', 'écris puis Entrée', 'escribe y pulsa Intro', 'escreve e prime Enter'],
  'ens.pw.saving': ['salvataggio…', 'saving…', 'wird gespeichert…', 'enregistrement…', 'guardando…', 'a guardar…'],
  'ens.pw.removing': ['cancellazione…', 'removing…', 'wird entfernt…', 'suppression…', 'borrando…', 'a remover…'],

  'ens.svc.on': ['Attivo', 'On', 'Aktiv', 'Activé', 'Activo', 'Ativo'],
  'ens.k.LookupToQso': ['Aggiungi nome e QTH ai QSO', 'Add name and QTH to QSOs', 'Name und QTH zu QSOs hinzufügen', 'Ajouter nom et QTH aux QSO', 'Añadir nombre y QTH a los QSO', 'Juntar nome e QTH aos QSO'],
  'jxh.LookupToQso': [
    'Con una ricerca accesa (QRZ.com o HamQTH), ogni QSO registrato riceve NAME (nome e cognome) e QTH (città), presi dalla memoria o con una ricerca al momento: se la risposta non arriva entro 2,5 secondi il QSO si registra senza. Valgono per il log, l’esportazione ADIF, gli invii diretti e l’ADIF via UDP. Nessun altro dato personale.',
    'With a lookup on (QRZ.com or HamQTH), each logged QSO gets NAME (first and last name) and QTH (city), from memory or a lookup on the spot: if no answer comes within 2.5 seconds the QSO is logged without. They go to the log, ADIF export, direct uploads and ADIF over UDP. No other personal data.',
    'Mit eingeschalteter Suche (QRZ.com oder HamQTH) erhält jedes geloggte QSO NAME (Vor- und Nachname) und QTH (Ort), aus dem Speicher oder per Suche im Moment: kommt die Antwort nicht binnen 2,5 Sekunden, wird ohne geloggt. Gilt für Log, ADIF-Export, direkte Uploads und ADIF per UDP. Keine weiteren persönlichen Daten.',
    'Avec une recherche activée (QRZ.com ou HamQTH), chaque QSO enregistré reçoit NAME (prénom et nom) et QTH (ville), depuis la mémoire ou par une recherche immédiate : sans réponse en 2,5 secondes, le QSO est enregistré sans. Valable pour le log, l’export ADIF, les envois directs et l’ADIF par UDP. Aucune autre donnée personnelle.',
    'Con una búsqueda activa (QRZ.com o HamQTH), cada QSO registrado recibe NAME (nombre y apellido) y QTH (ciudad), de la memoria o con una búsqueda al momento: si no hay respuesta en 2,5 segundos se registra sin ellos. Valen para el log, la exportación ADIF, los envíos directos y el ADIF por UDP. Ningún otro dato personal.',
    'Com uma pesquisa ligada (QRZ.com ou HamQTH), cada QSO registado recebe NAME (nome e apelido) e QTH (cidade), da memória ou com uma pesquisa no momento: se a resposta não chegar em 2,5 segundos regista-se sem eles. Valem para o log, a exportação ADIF, os envios diretos e o ADIF por UDP. Nenhum outro dado pessoal.',
  ],

  // spia di stato nel titolo dei riquadri
  'ens.led.off': ['spento o non configurato', 'off or not configured', 'aus oder nicht eingerichtet', 'désactivé ou non configuré', 'apagado o sin configurar', 'desligado ou não configurado'],
  'ens.led.untested': ['configurato, da provare', 'configured, not tested yet', 'eingerichtet, noch nicht getestet', 'configuré, à tester', 'configurado, sin probar', 'configurado, por testar'],
  'ens.led.ok': ['funziona (ultima prova o invio riuscito)', 'working (last test or send succeeded)', 'funktioniert (letzter Test oder Versand erfolgreich)', 'fonctionne (dernier test ou envoi réussi)', 'funciona (última prueba o envío correctos)', 'funciona (último teste ou envio com êxito)'],
  'ens.led.err': ['errore nell’ultima prova o invio', 'last test or send failed', 'letzter Test oder Versand fehlgeschlagen', 'échec du dernier test ou envoi', 'falló la última prueba o envío', 'o último teste ou envio falhou'],

  // rimandi fra le due pagine
  'ens.qrz.seeOnline': [
    'La ricerca dei nominativi su QRZ.com è in Servizi online.',
    'Callsign lookup on QRZ.com is in Online services.',
    'Die Rufzeichensuche auf QRZ.com ist unter Online-Dienste.',
    'La recherche d’indicatifs sur QRZ.com est dans Services en ligne.',
    'La búsqueda de indicativos en QRZ.com está en Servicios en línea.',
    'A pesquisa de indicativos no QRZ.com está em Serviços online.',
  ],
  'ens.qrz.seeLog': [
    'L’invio dei QSO al logbook di QRZ.com è in Servizi di log.',
    'Sending QSOs to the QRZ.com logbook is in Log services.',
    'Das Senden der QSOs ins QRZ.com-Logbuch ist unter Log-Dienste.',
    'L’envoi des QSO vers le carnet QRZ.com est dans Services de log.',
    'El envío de QSO al libro de QRZ.com está en Servicios de log.',
    'O envio dos QSO para o logbook do QRZ.com está em Serviços de log.',
  ],

  // ricerca QRZ.com
  'ens.g.qrzlookup': ['Ricerca QRZ.com', 'QRZ.com lookup', 'QRZ.com-Suche', 'Recherche QRZ.com', 'Búsqueda QRZ.com', 'Pesquisa QRZ.com'],
  'ens.up.qrzlookup.help': [
    'Quando passi il mouse su un nominativo nel DX Cluster, Aurora chiede a QRZ.com nome e città di quella stazione. Servono utente e password di QRZ.com (non la chiave del logbook). Con l’abbonamento «XML Logbook Data» QRZ dà i dati completi; senza, dà solo dati ridotti (spesso solo nome e paese). Di chi cerchi si mostrano solo nome, città e paese, mai email o indirizzo. I risultati restano in memoria tre giorni; spenta, non parte nessuna richiesta. «Prova» apre una sessione e dice se c’è l’abbonamento.',
    'When you hover a callsign in the DX Cluster, Aurora asks QRZ.com for that station’s name and city. It needs your QRZ.com username and password (not the logbook key). With the “XML Logbook Data” subscription QRZ returns full data; without it only reduced data (often just name and country). Only name, city and country are shown, never e-mail or address. Results are kept in memory for three days; switched off, no request is made. “Test” opens a session and tells whether the subscription is active.',
    'Fährst du im DX Cluster mit der Maus über ein Rufzeichen, fragt Aurora QRZ.com nach Name und Ort dieser Station. Nötig sind Benutzername und Passwort von QRZ.com (nicht der Logbuch-Schlüssel). Mit dem Abo „XML Logbook Data“ liefert QRZ alle Daten, ohne nur reduzierte (oft nur Name und Land). Gezeigt werden nur Name, Ort und Land, nie E-Mail oder Adresse. Ergebnisse bleiben drei Tage im Speicher; ausgeschaltet geht keine Anfrage raus. „Testen“ öffnet eine Sitzung und sagt, ob das Abo aktiv ist.',
    'Quand tu survoles un indicatif dans le DX Cluster, Aurora demande à QRZ.com le nom et la ville de cette station. Il faut l’utilisateur et le mot de passe QRZ.com (pas la clé du carnet). Avec l’abonnement « XML Logbook Data » QRZ donne toutes les données ; sans, seulement des données réduites (souvent nom et pays). Seuls nom, ville et pays sont affichés, jamais e-mail ou adresse. Les résultats restent en mémoire trois jours ; désactivée, aucune requête ne part. « Tester » ouvre une session et dit si l’abonnement est actif.',
    'Al pasar el ratón sobre un indicativo en el DX Cluster, Aurora pide a QRZ.com el nombre y la ciudad de esa estación. Hacen falta usuario y contraseña de QRZ.com (no la clave del libro). Con la suscripción «XML Logbook Data» QRZ da los datos completos; sin ella solo datos reducidos (a menudo solo nombre y país). Solo se muestran nombre, ciudad y país, nunca correo o dirección. Los resultados quedan en memoria tres días; apagada, no sale ninguna petición. «Probar» abre una sesión y dice si hay suscripción.',
    'Ao passar o rato sobre um indicativo no DX Cluster, a Aurora pede ao QRZ.com o nome e a cidade dessa estação. São precisos utilizador e palavra-passe do QRZ.com (não a chave do logbook). Com a assinatura «XML Logbook Data» o QRZ dá os dados completos; sem ela só dados reduzidos (muitas vezes só nome e país). Só se mostram nome, cidade e país, nunca email ou morada. Os resultados ficam em memória três dias; desligada, não sai nenhum pedido. «Testar» abre uma sessão e diz se há assinatura.',
  ],
  'ens.k.QRZLookup': ['Cerca nome e città su QRZ.com', 'Look up name and city on QRZ.com', 'Name und Ort auf QRZ.com suchen', 'Chercher nom et ville sur QRZ.com', 'Buscar nombre y ciudad en QRZ.com', 'Procurar nome e cidade no QRZ.com'],
  'ens.k.QRZUser': ['Utente QRZ.com', 'QRZ.com username', 'QRZ.com-Benutzer', 'Utilisateur QRZ.com', 'Usuario de QRZ.com', 'Utilizador QRZ.com'],
  'ens.k.QRZPasswd': ['Password QRZ.com', 'QRZ.com password', 'QRZ.com-Passwort', 'Mot de passe QRZ.com', 'Contraseña de QRZ.com', 'Palavra-passe QRZ.com'],
  'jxh.QRZLookup': [
    'Accende la ricerca di nome e città sul nominativo al passaggio del mouse. Spenta, Aurora non contatta QRZ.com.',
    'Turns on the name and city lookup when hovering a callsign. Off, Aurora never contacts QRZ.com.',
    'Schaltet die Suche nach Name und Ort beim Überfahren eines Rufzeichens ein. Aus: Aurora kontaktiert QRZ.com nie.',
    'Active la recherche du nom et de la ville au survol d’un indicatif. Désactivée, Aurora ne contacte jamais QRZ.com.',
    'Activa la búsqueda de nombre y ciudad al pasar sobre un indicativo. Apagada, Aurora nunca contacta con QRZ.com.',
    'Liga a pesquisa de nome e cidade ao passar sobre um indicativo. Desligada, a Aurora nunca contacta o QRZ.com.',
  ],
  'jxh.QRZUser': ['Il nome con cui entri su QRZ.com (di solito il tuo nominativo).', 'The name you sign in to QRZ.com with (usually your callsign).', 'Der Name, mit dem du dich bei QRZ.com anmeldest (meist dein Rufzeichen).', 'Le nom avec lequel tu te connectes à QRZ.com (souvent ton indicatif).', 'El nombre con el que entras en QRZ.com (normalmente tu indicativo).', 'O nome com que entras no QRZ.com (normalmente o teu indicativo).'],
  'jxh.QRZPasswd': ['La password di QRZ.com. Resta salvata solo su questo PC, protetta da Windows, e non si rilegge mai.', 'Your QRZ.com password. Stored only on this PC, protected by Windows, and never shown again.', 'Dein QRZ.com-Passwort. Nur auf diesem PC gespeichert, von Windows geschützt, und nie wieder angezeigt.', 'Ton mot de passe QRZ.com. Enregistré seulement sur ce PC, protégé par Windows, jamais réaffiché.', 'Tu contraseña de QRZ.com. Se guarda solo en este PC, protegida por Windows, y no se vuelve a mostrar.', 'A tua palavra-passe do QRZ.com. Fica só neste PC, protegida pelo Windows, e nunca mais é mostrada.'],

  // ricerca HamQTH
  'ens.g.hamqth': ['Ricerca HamQTH', 'HamQTH lookup', 'HamQTH-Suche', 'Recherche HamQTH', 'Búsqueda HamQTH', 'Pesquisa HamQTH'],
  'ens.up.hamqth.help': [
    'HamQTH è un archivio di nominativi gratuito: basta registrarsi su hamqth.com. Aurora lo usa se la ricerca QRZ.com è spenta o non trova il nominativo: per chi non ha l’abbonamento XML di QRZ è il modo per avere nome e città. Stesse regole: solo nome, città e paese, risultati in memoria tre giorni, nessuna richiesta se spenta. «Prova» apre una sessione.',
    'HamQTH is a free callsign database: just register on hamqth.com. Aurora uses it when the QRZ.com lookup is off or does not find the callsign: without a QRZ XML subscription it is the way to get name and city. Same rules: only name, city and country, results kept for three days, no request when off. “Test” opens a session.',
    'HamQTH ist eine kostenlose Rufzeichen-Datenbank: Registrierung auf hamqth.com genügt. Aurora nutzt sie, wenn die QRZ.com-Suche aus ist oder das Rufzeichen nicht findet: ohne QRZ-XML-Abo der Weg zu Name und Ort. Gleiche Regeln: nur Name, Ort und Land, Ergebnisse drei Tage gespeichert, ausgeschaltet keine Anfrage. „Testen“ öffnet eine Sitzung.',
    'HamQTH est une base d’indicatifs gratuite : il suffit de s’inscrire sur hamqth.com. Aurora l’utilise quand la recherche QRZ.com est désactivée ou ne trouve pas l’indicatif : sans abonnement XML QRZ, c’est le moyen d’avoir nom et ville. Mêmes règles : seulement nom, ville et pays, résultats gardés trois jours, aucune requête si désactivée. « Tester » ouvre une session.',
    'HamQTH es una base de indicativos gratuita: basta registrarse en hamqth.com. Aurora la usa si la búsqueda de QRZ.com está apagada o no encuentra el indicativo: sin suscripción XML de QRZ es la forma de tener nombre y ciudad. Mismas reglas: solo nombre, ciudad y país, resultados tres días en memoria, ninguna petición si está apagada. «Probar» abre una sesión.',
    'O HamQTH é uma base de indicativos gratuita: basta registar-se em hamqth.com. A Aurora usa-o se a pesquisa QRZ.com estiver desligada ou não encontrar o indicativo: sem assinatura XML do QRZ é a forma de ter nome e cidade. Mesmas regras: só nome, cidade e país, resultados três dias em memória, nenhum pedido se desligada. «Testar» abre uma sessão.',
  ],
  'ens.k.HamQTHLookup': ['Cerca nome e città su HamQTH', 'Look up name and city on HamQTH', 'Name und Ort auf HamQTH suchen', 'Chercher nom et ville sur HamQTH', 'Buscar nombre y ciudad en HamQTH', 'Procurar nome e cidade no HamQTH'],
  'ens.k.HamQTHUser': ['Utente HamQTH', 'HamQTH username', 'HamQTH-Benutzer', 'Utilisateur HamQTH', 'Usuario de HamQTH', 'Utilizador HamQTH'],
  'ens.k.HamQTHPasswd': ['Password HamQTH', 'HamQTH password', 'HamQTH-Passwort', 'Mot de passe HamQTH', 'Contraseña de HamQTH', 'Palavra-passe HamQTH'],
  'jxh.HamQTHLookup': ['Accende la ricerca su HamQTH (usata dopo QRZ.com). Spenta, Aurora non contatta HamQTH.', 'Turns on the HamQTH lookup (used after QRZ.com). Off, Aurora never contacts HamQTH.', 'Schaltet die HamQTH-Suche ein (nach QRZ.com). Aus: Aurora kontaktiert HamQTH nie.', 'Active la recherche HamQTH (après QRZ.com). Désactivée, Aurora ne contacte jamais HamQTH.', 'Activa la búsqueda en HamQTH (después de QRZ.com). Apagada, Aurora nunca contacta con HamQTH.', 'Liga a pesquisa no HamQTH (depois do QRZ.com). Desligada, a Aurora nunca contacta o HamQTH.'],
  'jxh.HamQTHUser': ['Il nome con cui entri su HamQTH (il tuo nominativo).', 'The name you sign in to HamQTH with (your callsign).', 'Der Name, mit dem du dich bei HamQTH anmeldest (dein Rufzeichen).', 'Le nom avec lequel tu te connectes à HamQTH (ton indicatif).', 'El nombre con el que entras en HamQTH (tu indicativo).', 'O nome com que entras no HamQTH (o teu indicativo).'],
  'jxh.HamQTHPasswd': ['La password di HamQTH. Resta salvata solo su questo PC, protetta da Windows.', 'Your HamQTH password. Stored only on this PC, protected by Windows.', 'Dein HamQTH-Passwort. Nur auf diesem PC gespeichert, von Windows geschützt.', 'Ton mot de passe HamQTH. Enregistré seulement sur ce PC, protégé par Windows.', 'Tu contraseña de HamQTH. Se guarda solo en este PC, protegida por Windows.', 'A tua palavra-passe do HamQTH. Fica só neste PC, protegida pelo Windows.'],

  // nominativo registrato su QRZ
  'ens.k.QRZLogCall': ['Nominativo registrato su QRZ (facoltativo)', 'Callsign registered on QRZ (optional)', 'Bei QRZ registriertes Rufzeichen (optional)', 'Indicatif enregistré sur QRZ (facultatif)', 'Indicativo registrado en QRZ (opcional)', 'Indicativo registado no QRZ (opcional)'],
  'jxh.QRZLogCall': [
    'Se lo scrivi, i QSO fatti con prefissi o suffissi (/P, /QRP, EA3/…) partono verso QRZ con questo nominativo: QRZ scarta i QSO con un nominativo diverso da quello del logbook. Vuoto = quello del QSO.',
    'If set, QSOs made with prefixes or suffixes (/P, /QRP, EA3/…) go to QRZ with this callsign: QRZ rejects QSOs whose callsign differs from the logbook’s. Empty = the QSO’s own.',
    'Wenn gesetzt, gehen QSOs mit Präfix oder Suffix (/P, /QRP, EA3/…) mit diesem Rufzeichen an QRZ: QRZ verwirft QSOs mit anderem Rufzeichen als dem des Logbuchs. Leer = das des QSO.',
    'Si rempli, les QSO faits avec préfixe ou suffixe (/P, /QRP, EA3/…) partent vers QRZ avec cet indicatif : QRZ rejette les QSO dont l’indicatif diffère de celui du carnet. Vide = celui du QSO.',
    'Si lo escribes, los QSO hechos con prefijos o sufijos (/P, /QRP, EA3/…) van a QRZ con este indicativo: QRZ descarta los QSO con un indicativo distinto del libro. Vacío = el del QSO.',
    'Se o escreveres, os QSO feitos com prefixos ou sufixos (/P, /QRP, EA3/…) vão para o QRZ com este indicativo: o QRZ rejeita QSO com indicativo diferente do logbook. Vazio = o do QSO.',
  ],

  // HRDLOG.NET
  'ens.g.hrdlog': ['Invio diretto a HRDLOG.NET', 'Direct upload to HRDLOG.NET', 'Direkter Upload zu HRDLOG.NET', 'Envoi direct vers HRDLOG.NET', 'Envío directo a HRDLOG.NET', 'Envio direto para o HRDLOG.NET'],
  'ens.up.hrdlog.help': [
    'Ogni QSO registrato parte subito verso il tuo log su HRDLOG.NET, con la stessa coda e gli stessi nuovi tentativi degli altri servizi. Servono il nominativo del log e il codice di caricamento («upload code») che HRDLOG.NET mostra nel tuo profilo. HRDLOG.NET non ha una prova senza invio: «Prova» controlla solo che i dati ci siano; il codice si verifica al primo QSO.',
    'Each logged QSO goes straight to your log on HRDLOG.NET, with the same queue and retries as the other services. It needs the log callsign and the “upload code” HRDLOG.NET shows in your profile. HRDLOG.NET has no test without sending: “Test” only checks the data is there; the code is checked with the first QSO.',
    'Jedes geloggte QSO geht sofort in dein Log auf HRDLOG.NET, mit derselben Warteschlange und Wiederholungen wie die anderen Dienste. Nötig sind Log-Rufzeichen und der „Upload Code“ aus deinem HRDLOG.NET-Profil. HRDLOG.NET hat keinen Test ohne Senden: „Testen“ prüft nur, ob die Daten da sind; der Code wird beim ersten QSO geprüft.',
    'Chaque QSO enregistré part tout de suite vers ton log sur HRDLOG.NET, avec la même file et les mêmes nouvelles tentatives que les autres services. Il faut l’indicatif du log et le « upload code » affiché dans ton profil HRDLOG.NET. HRDLOG.NET n’a pas de test sans envoi : « Tester » vérifie seulement que les données sont là ; le code est vérifié au premier QSO.',
    'Cada QSO registrado sale enseguida hacia tu log en HRDLOG.NET, con la misma cola y reintentos que los demás servicios. Hacen falta el indicativo del log y el «upload code» que HRDLOG.NET muestra en tu perfil. HRDLOG.NET no tiene prueba sin envío: «Probar» solo comprueba que estén los datos; el código se verifica con el primer QSO.',
    'Cada QSO registado sai logo para o teu log no HRDLOG.NET, com a mesma fila e novas tentativas dos outros serviços. São precisos o indicativo do log e o «upload code» que o HRDLOG.NET mostra no teu perfil. O HRDLOG.NET não tem teste sem envio: «Testar» só verifica se os dados estão lá; o código é verificado no primeiro QSO.',
  ],
  'ens.k.HrdlogSend': ['Invia ogni QSO a HRDLOG.NET', 'Send each QSO to HRDLOG.NET', 'Jedes QSO an HRDLOG.NET senden', 'Envoyer chaque QSO à HRDLOG.NET', 'Enviar cada QSO a HRDLOG.NET', 'Enviar cada QSO para o HRDLOG.NET'],
  'ens.k.HrdlogCall': ['Nominativo del log', 'Log callsign', 'Log-Rufzeichen', 'Indicatif du log', 'Indicativo del log', 'Indicativo do log'],
  'ens.k.HrdlogCode': ['Codice di caricamento (upload code)', 'Upload code', 'Upload Code', 'Code de téléversement (upload code)', 'Código de carga (upload code)', 'Código de carregamento (upload code)'],
  'jxh.HrdlogSend': ['Manda ogni QSO registrato al tuo log su HRDLOG.NET.', 'Sends each logged QSO to your log on HRDLOG.NET.', 'Schickt jedes geloggte QSO in dein Log auf HRDLOG.NET.', 'Envoie chaque QSO enregistré à ton log sur HRDLOG.NET.', 'Envía cada QSO registrado a tu log en HRDLOG.NET.', 'Envia cada QSO registado para o teu log no HRDLOG.NET.'],
  'jxh.HrdlogCall': ['Il nominativo del log su HRDLOG.NET. Vuoto = il tuo nominativo.', 'The log callsign on HRDLOG.NET. Empty = your callsign.', 'Das Log-Rufzeichen auf HRDLOG.NET. Leer = dein Rufzeichen.', 'L’indicatif du log sur HRDLOG.NET. Vide = ton indicatif.', 'El indicativo del log en HRDLOG.NET. Vacío = tu indicativo.', 'O indicativo do log no HRDLOG.NET. Vazio = o teu indicativo.'],
  'jxh.HrdlogCode': ['Il codice che HRDLOG.NET mostra nel profilo. Resta salvato solo su questo PC, protetto da Windows.', 'The code HRDLOG.NET shows in your profile. Stored only on this PC, protected by Windows.', 'Der Code aus deinem HRDLOG.NET-Profil. Nur auf diesem PC gespeichert, von Windows geschützt.', 'Le code affiché dans ton profil HRDLOG.NET. Enregistré seulement sur ce PC, protégé par Windows.', 'El código que HRDLOG.NET muestra en tu perfil. Se guarda solo en este PC, protegido por Windows.', 'O código que o HRDLOG.NET mostra no perfil. Fica só neste PC, protegido pelo Windows.'],
} as const satisfies Record<string, Entry>;
