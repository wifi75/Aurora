# Changelog

## Non rilasciato
- **Comandi a JTDX molto piu' rapidi**: Abilita Tx, la scelta del messaggio Tx, il cambio modo, la frequenza Rx e il livello Pwr passano da una sessione PowerShell sempre aperta che ricorda per dieci minuti i controlli della finestra di JTDX. Misurato dal vivo: un comando scende da circa 1 secondo a circa 60 ms, la lettura periodica da circa 1 secondo a circa 460 ms. Due sessioni (comandi e letture) cosi' una lettura lenta non fa aspettare mai un comando; si riaprono da sole se si bloccano; si riscaldano all'avvio e ogni cinque minuti.
- **Priorita' tra piu' CQ nello stesso ciclo**: i messaggi di un ciclo si raccolgono per 350 ms e l'automazione risponde a **uno solo**, il migliore. Regola «Con piu' CQ nello stesso ciclo rispondi a»: la novita' piu' rara (prima la lista dei desiderati, poi paese nuovo, nuovo su banda, zona, griglia; a parita' il segnale piu' forte) oppure il segnale piu' forte. Il registro dice quale e' stato scelto e quali sono stati lasciati. Tetti e salvaguardie valgono per la risposta scelta.
- **Cambio banda: il modo della radio si rimette**: se la radio, cambiando banda, richiama il modo memorizzato per quella banda (per esempio USB al posto di USB dati), Aurora riscrive il modo di prima, solo se e' cambiato e se OmniRig lo permette per quella radio. Da provare con la radio.

## 0.10.0 - 2026-10-24
- **Telefono** (schermi fino a 640 px): la pagina Stazione diventa a schede (Attivita' di banda, CQ, Trasmissione, Comandi, Cascata, DX Cluster, Log), una alla volta a tutta larghezza e con la scelta ricordata; menu e testata compatti con scorrimento di lato; modo, banda, comandi e interruttori del QSO in griglie a colonne uguali con bersagli da 44 px; campi a 16 px (niente zoom); messaggi su due righe con il testo leggibile; STOP TX sopra la riga in fondo. Corretti anche i pannelli con elenco che si schiacciavano a zero su schermi stretti. Sopra i 640 px non cambia nulla.
- **Aggiornamento automatico, base**: il core scarica il pacchetto di installazione dalla release (anche da un repository privato col token), ne verifica la firma minisign e lo installa solo se valida; `POST /api/v1/update/install` (solo dal PC, mai in trasmissione) e pulsante «Installa ora» nella barra degli aggiornamenti, visibile solo quando il programma sa installare (`can_install`). Il guscio Windows che lo usa non c'e' ancora.
- **Automazione, filtro dei falsi decode**: prima di rispondere a un CQ si scartano i messaggi segnati «?» dal motore, i nominativi senza la forma di un nominativo e i segnali sotto -23 dB di stazioni mai sentite in sessione; il motivo compare nel registro delle decisioni. Non cambia l'elenco dei messaggi.
- **Automazione, risposta ai desiderati**: nuova regola «Rispondi anche ai CQ della lista dei desiderati» (spenta di default) che usa la stessa lista degli avvisi, con filtro di banda sulla frequenza della radio e tutte le salvaguardie (segnale minimo, tentativi, tetti, modo reale solo dal PC, trasmissione chiusa di default).
- **Servizi online a richiesta** (scheda Impostazioni > Servizi online, **spenti di default**): *PSK Reporter* (chi ha ricevuto il tuo nominativo nelle ultime 3 ore, con distanza dal tuo locatore e il piu' lontano) e *propagazione* da hamqsl.com (flusso solare, indici A e K, condizioni per gruppo di bande di giorno e di notte, con la banda in uso evidenziata). Nuova pagina Propagazione. Sono richieste di sola lettura e rare; ogni servizio dice cosa vede il sito; non si invia mai nulla di cio' che si riceve o trasmette.
- **Diagnosi automatica** nella pagina Stazione: avvisa quando l'orologio e' fuori oltre il 4 % del periodo (8 % = problema), l'ingresso audio e' saturo, muto o debole, manca il nominativo nel motore o non si decodifica nulla da 3 minuti; ognuno con il modo di sistemarlo. Sparisce quando e' tutto a posto.
- **Spot del cluster sulla cascata**: gli spot della banda in uso che cadono dentro la cascata (frequenza dello spot meno quella della radio) compaiono in basso con tacca e nominativo, su piu' righe se vicini, per dieci minuti; un clic porta la frequenza di ricezione (Rx) su quella dello spot. Solo ricezione, non trasmette.
- **Storico dei messaggi**: ogni messaggio ricevuto si ricorda (`data/history.sqlite`, 30 giorni, `AURORA_HISTORY_DAYS`) e si cerca nella nuova pagina Storico per nominativo o testo, banda, modo e periodo, con pagine da 100 e svuotamento su conferma (`GET/DELETE /api/v1/history`, `GET /api/v1/history/stats`). Non e' il log dei QSO e non vi finisce nulla.
- **Filtro «Nascondi i CQ di stazioni gia' collegate su questa banda»** nella scheda Filtri: gli altri messaggi di quelle stazioni restano visibili.
- **Livello RX**: indicatore del livello dell'audio in ingresso accanto al cursore Pwr (valore medio, picco, stato Muto/Basso/Giusto/Alto/Saturo) con avviso di saturazione dopo 3 secondi e di silenzio dopo 8; evento `audio_level` sul flusso. Idea dal confronto con Decodium, scritta da zero.
- **Volume di registrazione di Windows**: cursore e pulsante AUTO accanto all'indicatore Rx (`GET/PUT /api/v1/input-level`), sul dispositivo da cui viene la cascata, lo stesso di JTDX. AUTO scende subito se il segnale satura, sale piano se resta debole, non agisce in trasmissione e si spegne se muovi il cursore; la scelta si ricorda in `data/input-level.json`.
- **Licenza**: Aurora e' rilasciata con la GNU GPL versione 3 o successiva (`LICENSE`, `license` nei manifest, badge del README).
- Studio di Decodium 4.0 (GPL-3.0) e confronto delle funzioni che Aurora non ha: `docs/confronto-decodium.md`.
- **Aggiornamenti** (Impostazioni → Aggiornamenti): Aurora controlla ogni qualche ora l'ultima release sul repository GitHub (nuovo crate `updater`; repository e intervallo a scelta, anche le versioni di prova, token di sola lettura per un repository privato che non esce mai dal programma) e, se e' piu' nuova di quella in uso, mostra una barra in alto con note e link, la manda con un evento a tutte le pagine aperte e, con l'indirizzo ntfy, anche sul telefono; avvisa una volta sola per versione, anche dopo un riavvio. Rotte `GET/PUT /update` e `POST /update/check`. Non scarica ne' installa nulla: l'installazione automatica spetta al programma Windows (da fare).
- **Lista dei desiderati** (Impostazioni → Desiderati): nominativi, prefissi, paesi e griglie da collegare, con filtro di banda e attesa tra due avvisi. Il core guarda ogni messaggio decodificato e ogni spot del cluster (anche a pagina chiusa); alla prima apparizione una barra gialla in alto (con suono se gli avvisi sonori sono accesi, e notifica del browser se consentita), la riga evidenziata negli elenchi e, con un indirizzo **ntfy**, la notifica sul telefono. Rotte `GET/PUT /wishlist` e `POST /wishlist/test`; nuovo crate `wishlist`.
- **Mappa** (`#/mappa`): planisfero con i quadrati locatore gia' lavorati nel log di Log4OM2 (piu' QSO, piu' colore; filtro per banda), le stazioni che si ricevono ora dal loro locatore (CQ e risposte, colori delle etichette del motore) e la tua posizione. Rotella o + e − per ingrandire, si trascina. Nuova rotta `GET /map/worked` (aggregata nel database: 17.000 QSO diventano 1.500 righe). Il flusso dei dati ora sta nell'applicazione e non nella pagina della stazione: cambiando pagina i messaggi continuano ad arrivare.
- Interruttori del QSO allineati alla finestra di JTDX: pulsante AutoSeq con il livello scritto dal motore («AutoSeq 3») e casella RRR (senza spunta il Tx4 e' RR73, con la spunta RRR).
- Impostazioni → Generali: casella «Cambiando banda o modo svuota l'attivita' di banda» (accesa di base). Vale per i cambi fatti da Aurora, subito, e per quelli fatti nel motore; spenta l'elenco resta, e «Cancella» lo svuota a mano.
- Riavvio di Aurora: i dati arrivano subito. Prima lo stato di JTDX (frequenza, modo, DX) compariva solo a fine periodo, fino a 13-15 secondi dopo, e nel frattempo anche STOP e i comandi non partivano. Ora Aurora legge lo stato dalla finestra di JTDX in meno di un secondo e ricorda su disco indirizzo e identificativo di JTDX (`data/jtdx-peer.txt`), cosi' i comandi, compreso STOP, raggiungono JTDX subito; la pagina si riconnette in 2 secondi al massimo (prima 8). I messaggi decodificati restano quelli del periodo in corso: nessuna vecchia stazione ricompare.
- Le letture da JTDX con l'accessibilita' di Windows escono ora in UTF-8 (prima la console li rovinava: lo spazio non divisibile della frequenza diventava un carattere illeggibile).
- Barre Rx (verde) e Tx (rossa) sulla cascata larghe quanto il segnale del modo, dalla frequenza verso destra: FT8 50 Hz (piu' stretta), FT4 83 Hz; per gli altri modi resta la barra sottile.
- Messaggi Tx1-Tx6 come nel motore: Aurora legge dalla sua finestra i testi veri (rapporto e CQ come li ha scritti) e il pallino selezionato, cioe' da dove trasmette; con «Salta Tx1» si parte da Tx2 e la prima riga si vede barrata. I messaggi si scelgono a mano con un clic (`POST /commands/tx-message`): il motore trasmette da quello e poi prosegue da solo. La riga e' accesa forte mentre si trasmette e col solo bordo mentre aspetta il turno.
- Orologio UTC (HH:MM:SS) accanto alla barra del periodo, con la stessa grafica e la stessa misura della barra.
- Cambio banda piu' veloce: da circa 2,7 a circa 0,9 secondi (attese a scatti brevi, split toccato solo se serve). Il pulsante di banda o di modo scelto si accende subito e resta cosi' finche' il motore non conferma, senza tornare indietro e poi riavanzare; se il comando fallisce torna com'era. Vale anche per il doppio clic sul cluster.
- Tasto del periodo di trasmissione come nel motore: «Tx 00/30» o «Tx 15/45» (FT4: «Tx 00/15» o «Tx 7,5/22,5»), di due colori, un clic passa dall'uno all'altro.
- Cascata: una riga per pixel con scorrimento continuo verso il basso, righe mediate (circa 4 al secondo) e filtro orizzontale: niente piu' effetto serpente.
- Attivita' di banda: l'etichetta colorata resta dentro la sua colonna (testo lungo con i puntini); Log: carattere piu' grande.
- Stazione DX: la riga Tx in corso si riconosce dal tipo di messaggio (locatore, rapporto, R+rapporto, RR73, 73, CQ) anche se il testo non coincide col modello, e mostra il testo davvero trasmesso.
- Attivita' di banda: la tacca rossa delle chiamate verso di te non compare piu' sui messaggi di chiusura (RR73 e 73), a QSO concluso.
- Menu in alto e schede delle Impostazioni: la voce in cui ci si trova e' piena e in grassetto (anche `aria-current`).
- Tema scuro: pannelli con fondo pieno, senza vetro sfocato dietro al testo, e tabelle del cluster e del log a corpo intero: i nominativi non risultano piu' sfuocati.

## 0.9.0 - 2026-10-24
- **DX Cluster** (crate `dx-cluster`): client telnet verso server pubblici (catalogo verificato) o propri, con accesso col nominativo, riconnessione, paese dal `cty.dat`, filtri della vista e comandi solo di consultazione (`sh/`, `set/`...): mai spot o messaggi pubblici. Finestra a sinistra di Comandi (DE, Freq, DX, bandiera, Paese, Modo, Commenti, UTC), doppio clic = la radio va sulla frequenza esatta dello spot (`POST /commands/frequency`, solo dentro le bande dei radioamatori, mai in trasmissione). Scheda DX Cluster nelle impostazioni con LED di raggiungibilita' dei server (`POST /dx-cluster/test`), salvataggio automatico e spento di default.
- **Client NTP** (crate `clock-sync`): scarto dell'orologio dai server NTP, controllo periodico, correzione dell'ora di Windows (amministratore, solo rete di casa), indicatore «Orologio» e DT mediano (il «Med» di JTDX) nella barra di stato, scheda Orologio.
- **Impostazioni di JTDX** lette dal suo `JTDX.ini` (`GET /engine-config`, `AURORA_JTDX_INI`, senza password): UDP, radio, audio, sequenza, avvisi, avanzate. Si modificano dall'interfaccia (`POST /engine-config`: chiude JTDX, scrive il file con copia di sicurezza, lo riavvia; mai in trasmissione, solo rete di casa) con una barra «Applica e riavvia» / «Annulla» e le voci cambiate evidenziate.
- **Log di Log4OM2** in sola lettura (`GET /log4om`, `AURORA_LOG4OM_DB`): finestra Log accanto all'attivita' di banda, numero di QSO scelto in Impostazioni.
- Preferenze dell'interfaccia salvate sul server (`data/ui-settings.json`, separate per PC e telefono). Le schede Cluster e Orologio si salvano da sole.
- **Radio**: cambiando banda lo split della FT-2000 si spegne (OmniRig `SetSimplexMode` lo lasciava acceso); il cursore Pwr si regola anche a trasmissione chiusa, in basso la potenza massima come in JTDX; modo JT accanto a FT8 e FT4; comandi di ricezione di JTDX (Hound, Modo SWL, Monitora, Sinc, AGCc, Filtro, Ferma, Decodifica, Cancella) e barra del periodo.
- Doppio clic: la stazione scelta compare subito e resta finche' JTDX non la conferma; avviso se JTDX accetta i Reply solo per CQ e 73. Doppio Esc: libera la stazione da chiamare (Canc DX). Aprendo Aurora non si rivedono piu' le stazioni di prima (`AURORA_REPLAY_ON_CONNECT` ora spento).
- Interfaccia: Attivita' di banda a colonne con griglia su un rigo solo, barra di stato essenziale, modi e bande sulla prima riga, contrasti dei temi chiaro e scuro, frequenze scritte `21.074,0`, piede di pagina sottile.
- L'interfaccia non nomina piu' JTDX: nei testi (sei lingue) e nel codice dell'app si parla di Aurora e di «motore»; la rotta delle impostazioni del motore e' ora `/engine-config` (era `/jtdx-config`).
- DX Cluster: i filtri Banda e Modo sono a scelta multipla (menu a caselle leggibile in entrambi i temi, anche «La mia banda»); le scelte si ricordano.
- Tema scuro, Stazione DX: righe Tx1-Tx6 neutre con una sola tinta per la riga in corso, testo piu' nitido.
- Cambio banda: OmniRig non rende scrivibile `Freq` sulla FT-2000 (il comando pareva riuscito ma la radio restava ferma): ora si scrive `FreqA` e si attende la frequenza nuova.
- Lo script `avvia-sviluppo.ps1` apre la trasmissione (con `-Chiusa` resta chiusa) e collega JTDX.ini, il log di Log4OM2 e OmniRig. Avvio pulito: chiude le copie dell'avvio precedente, apre una sola finestra di Windows Terminal e tiene una sola copia di JTDX; il server di sviluppo (5173) parte solo con `-Sviluppo`.
- Cambio banda: aspetta fino a circa 3 secondi che OmniRig sia in linea e ritenta una volta. Cluster: frequenza senza decimale. Attivita' di banda: distanza senza parentesi quadre. Valori di `AcceptUDPReplyMessages` corretti (1 = solo CQ, 2 = CQ e 73, 3 = qualsiasi). L'avviso «buffer audio perso» compare una volta a ogni 10, 100, 1000 occorrenze.

## 0.8.1 - 2026-10-24
- Tasto Esc: ferma la trasmissione e spegne AutoTX e Abilita Tx, in tutte le pagine; si spegne in Impostazioni → Generali.
- Il doppio clic su un messaggio accende anche Abilita Tx (con la trasmissione aperta): con «1 QSO» JTDX lo spegne a ogni QSO finito e non richiamava piu' la stazione scelta.
- Blocco DX della scheda Trasmissione piu' stretto.
- Cursore Pwr verticale reso uguale in ogni browser (un cursore orizzontale ruotato): su alcuni browser il verso era invertito e portava il livello al minimo invece che al massimo. In alto e' sempre il 100%.

## 0.8.0 - 2026-10-24
- Pannello Comandi come JTDX: AutoSeq, 1 QSO, Risp B4, Bypassa, Salta Tx1, Periodo Tx, AutoTX e Abilita Tx; frequenze Tx/Rx con le frecce di copia e Tx/Rx Split; Canc DX e Gen.Msgs; CQ e messaggio libero. Nuove rotte `GET /controls` e `POST /commands/control` (elenco chiuso: niente accordo ne' log; Abilita Tx e AutoTX si accendono solo con la trasmissione aperta, spegnerli e' sempre permesso).
- Scheda Trasmissione in due blocchi (stazione DX con Tx1-Tx6, e interruttori del QSO) con il livello Pwr verticale a destra.
- Doppio clic su un messaggio: la stazione compare subito nel blocco DX e JTDX la chiama al momento giusto; l'opzione si spegne in Impostazioni → Generali.
- Corretto il cambio modo FT8/FT4: prima accendeva solo la casella senza far cambiare modo a JTDX; ora preme il pulsante e resta nella banda di prima, sulla frequenza che la tabella indica per il nuovo modo.
- Cambio banda e modo si vedono subito nella pagina, senza aspettare il prossimo stato di JTDX.
- STOP TX piu' piccolo e discreto a riposo, pieno in trasmissione; l'esito dei comandi e' una nota che sparisce da sola e non occupa spazio.
- Pulsanti di modo e di banda a misura fissa; FT8 viola e FT4 ambra, anche da spenti.

## 0.7.0 - 2026-10-24
- Guida in-app (`#/guida`): 11 argomenti brevi nelle sei lingue, raggiungibile dalla testata e dalla scheda Avanzate.
- Pagina Statistiche (`#/statistiche`): premi DXCC, zone CQ e ITU, quadranti, prefissi, QSO per banda e paesi mancanti, dal log ADIF.
- Elenco messaggi: i propri messaggi trasmessi compaiono come righe `Tx` (una per trasmissione), in ordine d'orario, senza risposta al doppio clic.

## 0.6.0 - 2026-10-24
- Sei lingue (italiano, inglese, tedesco, francese, spagnolo, portoghese) con selettore nella testata e un tooltip su ogni controllo; i testi stanno in tabelle per area (`app/src/i18n/strings`) e i test impediscono chiavi mancanti o segnaposto diversi.
- Impostazioni a 13 schede come JTDX: generali, radio, audio, sequenza, macro TX, segnalazioni, frequenze, notifiche, filtri, programmazione, avanzate, accesso, finestre, bande.
- Filtri dei messaggi (continenti, paesi, nominativi, solo CQ/RRR/RR73/73, non standard, proprio continente), preferenze di visualizzazione (miglia, paese, bandiera, distanza, DT), flag evidenzia/suono per ogni tipo di messaggio, macro TX (@ DX, & io, # rapporto).
- Frequenze di lavoro per banda e modo (FT8/FT4) modificabili e salvate; il cambio banda le usa. Programmazione delle bande per orario UTC (fino a 5 righe).
- Accesso: durata del codice di associazione decisa dall'operatore (1-60 minuti, predefinita 10, solo dal PC), accesso senza codice per le prove (solo rete di casa, mai da proxy, mai al riavvio, `AURORA_OPEN_ACCESS`), elenco e revoca dei dispositivi, codice con QR dalle impostazioni.
- Stazione: bandiere dei paesi, colonne a larghezza fissa che si adattano al pannello, etichette corte con il nome intero nel tooltip, stazione chiamata in evidenza, chiamate verso di te in rilievo con tacca rossa e avviso sonoro dedicato, doppio clic per rispondere (un clic seleziona soltanto), fase del QSO nel pannello Tx, DX e messaggio in trasmissione.
- Cascata ridimensionabile a mano anche da telefono; il trascinamento dei pannelli non li fa piu' sparire.
- Livello audio di uscita (cursore Pwr di JTDX) con il suggerimento sull'ALC: `GET/PUT /api/v1/output-level`, la modifica richiede la trasmissione aperta.
- Continente in ogni messaggio e nello stato; il simulatore e i test non contengono nominativi reali.
- Corretto: con l'accesso senza codice il flusso `/stream` veniva chiuso ogni secondo (il dispositivo virtuale non e' nell'archivio), cosi' sul telefono comparivano "Scollegato", lampeggio e nessun dato. Ora il flusso resta aperto finche' l'accesso senza codice e' acceso.

## 0.5.0 - 2026-10-24

## 0.5.0 - 2026-10-24
- `qso-engine`: automazione dei QSO (risponde ai CQ con novita'). Parte spenta e in prova a vuoto; un solo QSO attivo, tentativi per stazione, tetto di risposte di fila (si spegne), tetto giornaliero, dead-man senza dispositivi collegati, STOP TX la spegne. Registro delle decisioni con il motivo. API `GET/PUT /api/v1/automation`: dal telefono solo prova a vuoto o spegnimento, il modo reale solo dal PC con l'avviso confermato e `AURORA_ALLOW_TX=1`.
- Il motore JTDX annuncia `TxHalted` a ogni STOP TX (anche prima di conoscere JTDX).
- `qso-export`: ogni QSO registrato va nel log di Aurora e, se configurato, in ADIF su UDP (`AURORA_EXPORT_UDP`) o su file (`AURORA_EXPORT_FILE`).
- `GET /api/v1/station-info`: come e' avviata la stazione (sola lettura).
- Interfaccia: finestre (cascata, elenco, stazione, CQ) trascinabili in qualunque colonna o in basso e ridimensionabili, divisore tra colonne; disposizione ricordata. Elenco delle stazioni a sinistra come in JTDX (UTC, dB, DT, frequenza, messaggio, paese e distanza in km, righe colorate per tipo). Barra delle bande (solo visualizzazione). Aspetto di vetro traslucido con sfondo luminoso.
- Impostazioni a schede: Colori (le 17 categorie di JTDX piu' la risposta automatica, valori di partenza del tema scuro di JTDX), Finestre, Avvisi sonori, Trasmissione, Log esterni, Collegamenti. Pagina Automazioni.
- Comandi di trasmissione: CQ (TriggerCQ di JTDX, provato dal vivo: il bool a `true` abilita Tx), messaggio libero (13 caratteri; con invio solo a trasmissione aperta) e frequenza audio di trasmissione. API `commands/cq`, `commands/free-text`, `commands/tx-frequency`; cio' che trasmette richiede `AURORA_ALLOW_TX=1` e una via consentita. Pannello Trasmissione nell'interfaccia.
- Stato: stazione DX, rapporto, frequenze Tx/Rx e messaggio in trasmissione da JTDX; pannello con Tx1-Tx6 previsti (si accende quello in uso).
- Etichette complete come i colori di JTDX: zona CQ e ITU (anche per banda), griglia per banda, prefisso (WPX), nuova stazione, gia' lavorata.
- Finestre comprimibili con doppio clic sul titolo (la cascata si riduce alla sola barra).
- L'interfaccia compilata si serve da Aurora (`AURORA_UI_DIR`) anche sulla rete locale, per l'uso da smartphone.
- Cambio banda: `rig-control` comanda la radio via OmniRig (condiviso con JTDX, che lo usa come CAT), `POST /api/v1/commands/band` con le frequenze FT8 predefinite, mai in trasmissione; barra delle bande cliccabile. Si attiva con `AURORA_RIG=omnirig`. Provato dal vivo (15 m -> 20 m -> 15 m).
- Cambio modo FT8/FT4 (si preme il pulsante di JTDX con l'accessibilita' di Windows, `POST commands/mode`), frequenza Rx impostabile (`commands/rx-frequency`).
- Spettro con i colori di JTDX (blu, giallo, bianco) e le due barre verde (Rx) e rossa (Tx) trascinabili; clic sposta Rx, Maiusc+clic sposta Tx.
- Linguetta piccola a destra: rosso lampeggiante = trasmissione in onda, verde = ascolto. Barra delle bande filtrabile dalle impostazioni, banda in uso ben evidente.
- Statistiche del log (premi DXCC, zone CQ e ITU, griglie, prefissi, paesi mancanti), regole dell'automazione salvate.
- Simulatore e test senza nominativi reali.
- Test dell'interfaccia: cartelle temporanee uniche nei test e2e.

## 0.4.0 - 2026-10-24
- Cascata: contrasto automatico che segue il rumore di fondo.
- `dxcc`: paese e zone dei nominativi da `cty.dat` (prefissi piu' lunghi, eccezioni `=CALL`, `/P` e simili, `/MM` senza paese).
- Novita' sui decode: paese di chi trasmette e etichette NUOVO DXCC, DXCC NUOVA BANDA, NUOVA ZONA, NUOVA GRIGLIA, calcolate dai QSO gia' fatti (log ADIF di JTDX) e aggiornate a ogni QSO chiuso. Senza log o senza banda nota restano solo le etichette CQ.
- Nuove variabili: `AURORA_CTY_PATH`, `AURORA_WORKED_ADIF`; file mancanti non fermano Aurora.
- Interfaccia: pannello "CQ e novita'", paese sotto ogni messaggio, avvisi sonori (spenti all'inizio, suono diverso per nuovo paese e altre novita', volume ricordato).

## 0.3.0 - 2026-10-24
- Collegamento reale a JTDX 2.2.159: `udp-hub` (ripetitore del flusso UDP, inoltro identico a GridTracker2 e comandi di ritorno), `engine-jtdx` (Status, Decode e QSO negli eventi di Aurora; Reply e HaltTx verso JTDX).
- **Trasmissione chiusa per impostazione predefinita** (`AURORA_ALLOW_TX=0`): un Reply fa trasmettere JTDX davvero. HaltTx e STOP TX funzionano sempre.
- `audio-spectrum`: cascata dall'ingresso audio (`AURORA_AUDIO_DEVICE`).
- `udp-bridge`: Heartbeat di JTDX senza revisione, comando Replay; test con pacchetti reali anonimizzati.
- Documentazione: mappa dei flussi UDP (JTDX, GridTracker2, Log4OM2), guida al collegamento, esiti del reverse con REA e Ghidra.

## 0.2.0 - 2026-10-24
- Core: `aurora-core` (tipi, bus di eventi, trait `Engine`), `engine-sim` (stazione simulata), `logbook` (SQLite, duplicati, ADIF), `auth` (associazione dispositivi con token a hash e blocco anti-forza-bruta), `remote-api` (API v1, WebSocket, OpenAPI), programma `aurora`.
- Interfaccia: React 19 + TypeScript 7 + Vite 8, tema Notte radio, cascata WebGPU con ripiego WebGL2, pagina Live, associazione del dispositivo, installazione come app (PWA).
- Script `avvia-sviluppo.ps1` e `associa-telefono.ps1`.
- Reverse di `jtdx.exe` con REA + Ghidra: conferme su server UDP e Reply (vedi `docs/fase0-rapporto.md`).

## 0.0.2 - 2026-10-24
- `udp-bridge`: decodifica e codifica del protocollo UDP di JTDX.
- Strumenti di prova `udp_capture`, `udp_send`, `audio_tone`.
- Bozza dell'inventario delle funzioni di JTDX e guida alle prove manuali.

## 0.0.1 - 2026-10-24
- Struttura iniziale del repository e specifica di design.
