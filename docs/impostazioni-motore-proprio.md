# Impostazioni di Aurora con il motore proprio

Con `AURORA_ENGINE=native` (quello di tutti i giorni) **JTDX non c'e'**: scrivere il suo `JTDX.ini` e «riavviare il motore» non cambiava niente di vero. Le impostazioni
delle schede Generali, Radio, Audio, Sequenza, Segnalazioni, Notifiche e Avanzate sono quindi **di Aurora**.

## Come funzionano

- File `settings-motore.json` nella cartella dei dati (`%APPDATA%\Aurora`): `{ "version": 1, "values": { ... } }`, con copia di sicurezza `settings-motore.json.bak` (l'ultimo file buono) e
  scrittura atomica (file accanto + rinomina). Se il file e' rovinato si usa la copia; se mancano entrambe si riparte dai valori iniziali.
- Le chiavi hanno i nomi di `JTDX.ini` (si ritrovano nei suoi documenti), i valori sono di Aurora. Si tengono nel file **solo** le voci con un effetto vero.
- **Il `JTDX.ini` si legge una volta sola**, quando `settings-motore.json` non esiste, come valori iniziali (`import_from_ini`). Se il primo server UDP di JTDX era l'ascolto di Aurora
  stessa (`127.0.0.1:2334`, l'hub), il flusso verso GridTracker passava dall'inoltro: il primo server diventa il primo indirizzo di `AURORA_UDP_FORWARD`; senza, resta vuoto e l'invio dell'ADIF
  al primo server si spegne. Senza `JTDX.ini`: valori della schermata di JTDX (127.0.0.1, porta 2237, invii accesi, ALL.TXT dei decodificati) salvo i comandi UDP, spenti (un `Reply` fa trasmettere).
- API: `GET /api/v1/engine-settings` (valori e, per **ogni** voce di JTDX, `state: live|unavailable`, `apply: now|restart`, `reason`), `PUT /api/v1/engine-settings` `{changes}` (solo dalla rete di
  casa, `403 local_only`; `400 bad_setting` se un valore o una voce non e' ammesso, e non cambia niente; risponde con il documento aggiornato). `404 engine_settings_unavailable` con il motore JTDX,
  dove restano `GET/POST /api/v1/engine-config` (scrivono `JTDX.ini` e riavviano JTDX, invariati).
- Applicazione a caldo: il deposito avvisa chi si e' iscritto (`subscribe`, canale `watch`). Il server secondario cambia la lista dei destinatari dell'esportatore dei QSO a ogni QSO
  (`Exporter::spawn_shared`); il primo server riavvia **solo l'emettitore UDP** (`udp-emitter`, qualche istante). Nominativo e locatore valgono dal prossimo avvio (il trasmettitore li legge all'avvio).
- Interfaccia: la scheda Segnalazioni ha gli stessi cinque gruppi e le stesse voci della schermata di JTDX; le voci non disponibili si vedono **spente, con il motivo**; nelle altre schede le voci
  non disponibili stanno in un elenco «Non disponibili in Aurora» con il motivo. Con il motore JTDX resta il comportamento di prima.

## Regole

- **Anti-doppione del record ADIF**: i destinatari UDP sono quelli di `AURORA_EXPORT_UDP` (destinatari in piu' dell'ambiente) piu' il server secondario, senza indirizzi ripetuti
  (`export_udp_targets`). Se il secondo server coincide con il primo che riceve gia' l'ADIF (`LoggedADIF`, tipo 12), il datagramma semplice non parte. Il programma Windows non
  inietta piu' `AURORA_EXPORT_UDP` dal `JTDX.ini`: il secondo server si spegne dalla scheda. Il file `aurora_log.adi` si scrive sempre.
- Un invio acceso senza un indirizzo e una porta validi (IP o `localhost`, porta 1-65535) viene rifiutato: non si finge.
- Le credenziali di eQSL non si salvano mai.

## Tabella (chiave di `JTDX.ini`, significato in JTDX, cosa fa Aurora, stato)

Evidenza: **[ini]** = chiave e valore letti nel `JTDX.ini` dell'installazione e nella schermata di JTDX; **[REA]** = funzione letta in `jtdx.exe` 2.2.159
(`docs/analisi-jtdx-reverse-rea.md`, sezione «Registrazione dei QSO», e `docs/fase0-rapporto.md`); **[GPL]** = sorgenti pubblici; **[da verificare]** = non ancora riletto in `jtdx.exe`.

Nota sull'evidenza: la ricerca REA delle stringhe di `PSKReporter`, `DXSummit`, `toRTTY`, `dBtoComments`, `distanceToComments`, `ClearCallGridExit`, `WriteDecodedDebugALLTXT`, `EQSL*` in `jtdx.exe` e' andata
in timeout dell'analisi Ghidra (330 s) e un secondo tentativo e' stato interrotto per chiudere il lavoro: per queste voci le chiavi sono quelle del `JTDX.ini` e della schermata di JTDX (valori iniziali
inclusi), la funzione che le legge resta **da verificare con REA** (con `REA_GHIDRA_STARTUP_TIMEOUT_MS` piu' alto). Sono tutte disattivate in Aurora, quindi nessun comportamento e' stato indovinato.

### Segnalazioni

| Gruppo | Voce (chiave) | Significato in JTDX | Aurora | Stato |
|---|---|---|---|---|
| Registrazione log | Richiedimi di loggare il QSO (`PromptToLog`) | apre la finestra di conferma del log; senza, `acceptQSO` parte da solo. [REA] `guiUpdate` 0x4c4350, `initLogQSO` | manca la finestra di conferma; Aurora registra da sola | **spenta**, motivo `log_prompt`; da fare |
| | Messa a log automatica (`AutoQSOLogging`) | con `PromptToLog` spento il dialogo si accetta da solo (`autolog()`). [REA] | si registra sempre quando parte il 73 | **spenta** (sempre attiva), motivo `log_auto`; da fare lo spegnimento |
| | Converti il modo in RTTY (`toRTTY`) | scrive `MODE` RTTY nel record. [ini] [da verificare] | il record ADIF non lo prevede | **spenta**, `adif_options`; da fare |
| | Rapporto dB nel commento (`dBtoComments`) | aggiunge i rapporti al `COMMENT` ADIF. [ini] [da verificare] | `COMMENT` non c'e' | **spenta**, `adif_options`; da fare |
| | Distanza nei commenti (`distanceToComments`) | aggiunge la distanza al `COMMENT`. [ini] [da verificare] | idem | **spenta**, `adif_options`; da fare |
| | Cancella i dati dopo aver loggato (`ClearCallGrid`) | `logClearDXTimer` azzera DX Call e locatore dopo il log, solo senza AutoTX/AutoSeq. [REA] | la pagina ha le sue regole | **spenta**, `clear_dx`; da fare |
| | Cancella Nom DX e Griglia all'uscita (`ClearCallGridExit`) | alla chiusura di JTDX. [ini] | non ha senso allo stesso modo | **spenta**, `clear_dx` |
| Invia file ADIF del QSO per il log | Server/Porta/Abilita TCP (`TCPServer`, `TCPServerPort`, `EnableTCPConnection`) | altro percorso di consegna del log (comando `Log`). [REA] | Aurora non ha un server TCP: ha l'API HTTP/WebSocket e l'UDP | **spente**, `tcp`; da fare se serve |
| | Server UDP2, Porta UDP, Abilita (`UDP2Server`, `UDP2ServerPort`, `EnableUDP2adifBroadcast`) | un datagramma con il solo record ADIF a `UDP2Server:porta`, un invio per QSO (Log4OM2). [REA] `acceptQSO2` 0x472db0, `LogQSO::accept` 0x60be60 | destinatario dell'esportatore dei QSO, **a caldo**, con la regola anti-doppione | **funziona** |
| Registrazione su ALL.TXT | Messaggi decodificati (`WriteDecodedALLTXT`) | scrive le righe di decodifica in `ALL.TXT`. [ini] | ALL.TXT del motore proprio (`engine-native/src/alltxt.rs`): oggi si scrive sempre (`AURORA_ALL_TXT=0` lo spegne all'avvio) e l'interruttore condiviso non e' ancora collegato | **spenta** (`all_txt_pending`): si accende con `store.set_feature(Feature::AllTxt, true)` quando `AllTxt` legge `reports().all_txt` |
| | Messaggi decodificati e di debug (`WriteDecodedDebugALLTXT`) | righe di debug (priorita', «Wanted Call»...). [REA] stringhe di debug | non scritte | **spenta**, stesso motivo |
| Servizi di rete | eQSL (`EQSLSend`, `EQSLUser`, `EQSLPasswd`, `EQSLNick`) | invio del QSO a eQSL (`EQSL_QSL_SENT`). [REA] `acceptQSO2` | non realizzato; credenziali mai salvate | **spente**, `eqsl`; da fare |
| | Spotting PSK Reporter (`PSKReporter`) | JTDX manda le ricezioni (IPFIX su UDP) a PSK Reporter. [ini] [GPL] `psk_reporter.cpp` | invio non realizzato | **spenta**, `psk`; da fare (formato e destinatario da verificare in `jtdx.exe`) |
| | Spotting DXSummit (`AllowSpotsDXSummit`) | permette gli spot verso DXSummit. [ini] [da verificare] | non realizzato | **spenta**, `dxsummit`; da fare |
| Server UDP primario | Server UDP + Numero porta (`UDPServer`, `UDPServerPort`) | destinatario del flusso WSJT-X: Heartbeat, Status, Decode, Clear, QsoLogged, LoggedADIF. [REA] `MessageClient` | `forward_targets`: il primo server (piu' gli altri indirizzi di `AURORA_UDP_FORWARD`) e' il destinatario dell'emettitore UDP, che si riavvia a ogni modifica | **funziona** (con l'emettitore acceso: `AURORA_UDP_EMIT` non a 0) |
| | Accetta richieste UDP (`AcceptUDPRequests`) | accetta `Reply`, `HaltTx`, `FreeText`... in ritorno. [REA] `replyToUDP` 0x4778ab | `EmitterConfig.accept_requests`; di fabbrica **spento** (un `Reply` fa trasmettere), si importa da JTDX | **funziona** (emettitore) |
| | Risposte UDP (Reply) accettate (`AcceptUDPReplyMessages`) | 1 solo CQ, 2 CQ e 73, 3 qualsiasi (menu del programma). [REA] | `EmitterConfig.accept_reply` | **funziona** |
| | Notifica / Finestra di ripristino (`udpWindowToFront`, `udpWindowRestore`) | porta la finestra in primo piano o la ripristina a un comando. [REA] `replyToUDP` | non c'e' una finestra di JTDX | **spente**, `udp_window` |
| | Invio del file ADIF per il log (`EnableUDP1adifSending`) | dopo il `QsoLogged` (tipo 5) manda anche il `LoggedADIF` (tipo 12). [REA] 0x56a6d0 | `EmitterConfig.send_adif` | **funziona** (emettitore) |
| | Prevenire i nominativi non confermati (`preventFalseUDPspots`) | non esce via UDP un decode marcato `?` (`isWrong`). [REA] `postDecode` | il decodificatore di Aurora non produce quel marcatore: niente da escludere | **spenta**, `false_spots`; da rivedere con piu' dati |
| | Filtri di testo sui messaggi UDP (`ApplyFiltersToUDPmessages`) | solo le righe visibili dai filtri escono via UDP. [REA] `postDecode` 3952 | i filtri di Aurora agiscono sullo schermo | **spenta**, `udp_filters`; da fare |

### Altre schede

| Scheda | Voci | Aurora | Stato |
|---|---|---|---|
| Generali | `MyCall`, `MyGrid` | nominativo e locatore di Aurora: record ADIF, flusso UDP, trasmissione, DX Cluster, continente per i CQ direzionali | **funzionano** (dal prossimo avvio) |
| | `Language`, `Miles`, `showDistance`, `countryName`, `UseDarkStyle` | sono nella sezione Schermo, tema e lingua di Aurora | nascoste, `own_screen` |
| | `TxWatchdogTimer`, `TxQSYAllowed`, `MonitorOFF`, `callsignLogFiltering`, `gridLogFiltering`, `timeFromLogFiltering` | **schede Generali come in JTDX**: vedi «Scheda Generali» sotto | **funzionano** (a caldo; `MonitorOFF` al prossimo avvio) |
| | `Region`, `Type2MsgGen`, `Decode52`, `After73`, `IDint`, `TuneTimer`, `MonitorLastUsed`, `Scroll`, `HideTxMessages`, `countryPrefix`, `AlignDistanceAzimuth` | visibili e spente con il motivo (vedi sotto) | spente |
| | `InsertBlank`, `Tx2QSO`, `showAzimuth` | opzioni dello schermo, nel browser (valgono subito) | **funzionano** |
| Radio | `Rig`, `PTTport`, `CATSerialPort`, `CATSerialRate`, `Polling`, `CATForceDTR/RTS`, `TCIAudio` | la radio e il PTT stanno nella scheda CAT | nascoste, `radio_cat` |
| Audio | `SoundInName`, `SoundOutName`, `AudioInputChannel`, `AudioOutputChannel` | dispositivi scelti all'avvio (`AURORA_AUDIO_DEVICE`, `AURORA_AUDIO_OUT_DEVICE`) | nascoste, `audio_start`; da fare la scelta dal programma |
| | `SaveDir`, `SaveWav` | registrazione dei periodi solo con `AURORA_NATIVE_RECORD` | nascoste, `no_wav_save` |
| Sequenza | `Seq*Count`, `Seq*CounterValue` (quattro scenari), `SeqHaltTxReplyOther`, `StrictDirectionalCQ` | **scheda Sequenza come in JTDX**: vedi «Scheda Sequenza» sotto | **funzionano** (a caldo) |
| | `AutoSequence`, `SingleShotQSO`, `SkipTx1`, `AnswerWorkedB4`, `CallPriorityCQ`, `MaxDistance` | sono i pulsanti della finestra principale e le Regole di Aurora (pagina Automazioni) | nascoste, `own_rules` |
| Notifiche | `newCQZ/ITUZ/DXCC/Grid/Px/Call` (+ `Band`, `BandMode`), `newPotential`, `73RR73Marker` | **scheda Notifiche come in JTDX**: vedi «Scheda Notifiche» sotto | **funzionano** (a caldo) |
| | `workedColor`, `workedStriked`, `workedUnderlined`, `workedDontShow`, `txtColor`, `OtherStandardMessagesMarker`, `redMarker`, `blueMarker`, `differentBackground`, `highlightDXCall`... | come si disegnano i messaggi: scelte del browser, per dispositivo, valgono subito | nascoste, `own_notify` (nel documento API); **funzionano** nell'interfaccia |
| | `BeepOn*` | «anche un suono»: i suoni per tipo di messaggio di Aurora | **funzionano** (nel browser) |
| | `AlertOn*` (e `OB`) | un file audio di avviso a parte: Aurora ha i suoi suoni | nascoste, `own_alerts` |
| Avanzate | `NDepth`, `NFT4Depth`, `NFT8Cycles`, `FT8Sensitivity`, ... | il decodificatore di Aurora ha parametri suoi | nascoste, `own_decoder` |
| Macro TX, Frequenze, Filtri, Programmazione | — | gia' di Aurora (non scrivevano nel `JTDX.ini`) | invariate |

## Le schede Sequenza, Generali e Notifiche come in JTDX (2026-10-25)

Le tre schede ricalcano la schermata di JTDX voce per voce. Evidenza: **[ini]** = chiave e valore nel `JTDX.ini` dell'installazione; **[GPL]** = sorgenti pubblici (`Configuration.cpp`, `displaytext.cpp`,
`mainwindow.cpp`, `logbook/*.cpp`; riga indicata); **[binario]** = funzione letta in `jtdx.exe` 2.2.159 (nome e indirizzo; i simboli ci sono). Dove **[binario]** manca, vale solo il sorgente (che nei punti
controllati coincide con l'eseguibile).

### Scheda Sequenza

| Voce | Chiave | Valore di fabbrica in JTDX | Funzione che la legge | In Aurora |
|---|---|---|---|---|
| risposta al CQ di una chiamata senza risposta, N volte | `SeqAnswerCQCount` + `SeqAnswerCQCounterValue` (1-5) | spento, 2 | `Configuration::read_settings` [GPL 2246/2252] | `RepeatLimits.answer_cq`: dopo N trasmissioni dello stesso messaggio la successiva non parte e la sequenza cerca altro; non vale per stazioni volute e nuovi DXCC |
| chiamata CQ, ricevi una chiamata e rispondi, N volte | `SeqAnswerInCallCount` + `...CounterValue` | spento, 3 | idem [GPL 2247/2253] | `RepeatLimits.answer_in_call` |
| invia il rapporto di risposta al rapporto ricevuto, N volte | `SeqSentRReportCount` + `...CounterValue` | spento, 5 | idem [GPL 2248/2254] | `RepeatLimits.sent_r_report` |
| invia RR73 (73) in risposta a RRR (RR73), N volte | `SeqSentRR7373Count` + `...CounterValue` | spento, 4 | idem [GPL 2249/2255] | `RepeatLimits.closing` |
| ferma il Tx se l'operatore chiamato ha risposto ad un altro | `SeqHaltTxReplyOther` | **acceso** | `Configuration::halttxreplyother` (letto da `readFromStdout`) [GPL 2257] | `Options.abandon_if_dx_answers_other`; spento ferma solo se il DX parla entro la banda di guardia (FT8 51 Hz, FT4 84 Hz) |
| operazione CQ rigorosa | `StrictDirectionalCQ` | spento | `Configuration::strictdirCQ` [GPL 2256] | `Options.strict_directional_cq`: la direzione del CQ (CQ NA, CQ DX...) vale anche per desiderati e nuovi DXCC |

Dalle impostazioni la modifica arriva alla sequenza **a caldo** (`NativeControls::set_sequence`, comando `OptionChange::Sequence`, nessun riavvio) e all'avvio (`TxDriver` legge le impostazioni date prima della partenza).
Valori di partenza di Aurora (file nuovo): il rapporto R e la chiusura **accesi a 5**, i primi due spenti con i valori 2 e 3: e' la configurazione che Aurora ha sempre avuto e quella del proprietario; da un `JTDX.ini` si importano i valori veri una volta.
Il valore e' limitato a 1-5 (come in JTDX, che altrimenti torna al valore di fabbrica). La semantica completa dei contatori e' in `docs/logiche-jtdx.md`, sezione 2.5.

### Scheda Generali

| Voce | Chiave | Funzione che la legge | In Aurora |
|---|---|---|---|
| Mio nominativo, Mio locatore | `MyCall`, `MyGrid` | `Configuration::read_settings` [GPL 2169-2170] | **vivi**, dal prossimo avvio |
| Regione IARU | `Region` ([ini] valore binario `@Variant(IARURegions::Region)`) | `Configuration::region` [GPL 2455]: sceglie la tabella delle frequenze per regione | **spenta** (`iaru_region`): Aurora ha le sue frequenze |
| Generazione dei messaggi per nominativi composti di tipo 2 | `Type2MsgGen` ([ini] `@Variant`, valori «Nom completo in Tx1/Tx3/Tx5») | `Configuration::type_2_msg_gen`; il suggerimento di JTDX dice che riguarda JT65 e JT9 [GPL `Configuration.ui`] | **spenta** (`type2_msgs`): FT8 e FT4 trasportano i nominativi composti in un altro modo |
| Usa questo nominativo / questa griglia / filtra il LOG a partire dalla data/ora | `callsignLogFiltering`, `gridLogFiltering`, `timeFromLogFiltering` | `ADIF::load(mycall, mygrid, mydate)` [GPL adif.cpp 106-108; **binario** `ADIF::load` 0x5f01c0: confronto `STATION_CALLSIGN` uguale, `MY_GRIDSQUARE` primi 4 caratteri, `mydate <= QSO_DATE+TIME_ON` con l'ora completata a 14 cifre] | **vivi**, a caldo: il log (interno, `wsjtx_log.adi`, Log4OM2) si rilegge col filtro. Il log interno e Log4OM2 non dicono con quale nominativo/griglia si e' operato: per quelli il filtro vale solo per la data |
| Riga vuota tra i periodi di decodifica | `InsertBlank` | `readFromStdout`, `p1ReadFromStdout` [**binario** 0x494220, 0x49d680; GPL mainwindow.cpp 3792] | **viva** (schermo, `insertBlank`): uno spazio tra un periodo e il successivo |
| Usa il tema scuro | `UseDarkStyle` | `Configuration::useDarkStyle` | il tema di Aurora (pulsante tema, nella stessa sezione) |
| Distanza in miglia | `Miles` | `Configuration::miles` | **viva** (schermo) |
| Mostra il nome del paese / il prefisso | `countryName` / `countryPrefix` | `DisplayText::setConfiguration` [**binario** 0x4eaa30] | il nome: **vivo**; il prefisso al posto del nome: **spento** (`screen_off`) |
| Show distance / azimuth in messages with grid; Align DXCC / distance / azimuth | `showDistance`, `showAzimuth`, `AlignDistanceAzimuth` | `Configuration::showAzimuth`, `align`, `align_steps` | distanza e azimut: **vivi** (schermo); l'allineamento: **spento** (`screen_off`) |
| Nascondi i msg parzialmente trasmessi / Visualizza i tuoi msg Tx nella finestra Rx / Scorri in basso | `HideTxMessages`, `Tx2QSO`, `Scroll` | `Configuration::hide_TX_messages`, `TX_messages`, `scroll` | `Tx2QSO`: **vivo** (le righe Tx dell'elenco); gli altri due **spenti** (`screen_off`) |
| Carattere dell'applicazione / del testo ricevuto | — | — | **spenti** (`screen_off`) |
| Non monitorare all'avvio | `MonitorOFF` | `MainWindow::MainWindow` 0x4c9ca0 e `handle_transceiver_update` 0x4ad430 [**binario**]: all'avvio spegne Monitor | **viva**, al prossimo avvio (`monitor.store(false)` prima di partire) |
| Decodifica a t = 52 s | `Decode52` | letta solo da `setStopHSym` 0x47ae80 e dalle azioni JT9, T10, JT65, JT9+JT65 [**binario**] | **spenta** (`decode52`): vale solo per i modi JT |
| Consenti cambi di frequenza Tx durante la trasmissione | `TxQSYAllowed` | `transmitDisplay` 0x49d210, `setRig` 0x4bdb10, `guiUpdate` 0x4c4350 [**binario**]; spento disabilita il campo Tx durante la trasmissione e `setRig` non cambia la radio | **viva**: spento, `SetTxFreq` si ignora mentre si trasmette (di fabbrica acceso, come il valore del proprietario; in JTDX senza la chiave vale spento) |
| La ricezione ritorna sull'ultima frequenza usata | `MonitorLastUsed` | solo `on_monitorButton_clicked` 0x4acf00 [**binario**]: riaccendendo Monitor riporta la radio sull'ultima frequenza ascoltata | **spenta** (`monitor_last`): da fare (ricordare la frequenza e riscriverla alla radio) |
| Invia ID CW dopo 73; Intervallo periodo ID CW | `After73`, `IDint` | solo `guiUpdate` 0x4c4350, dentro `!m_mode.startsWith("FT")` [**binario**] | **spente** (`cw_id`): l'ID CW non esiste in FT8 e FT4 |
| Periodo monitoraggio Tx (minuti) | `TxWatchdogTimer` (0-99, fabbrica 6) | `on_the_minute` 0x48aa40, `update_watchdog_label`, `guiUpdate` [**binario**] | **viva**: `Options.watchdog_min` (a caldo) |
| Tempo per l'accordo | `TuneTimer` (0-300, fabbrica 30) | `on_tuneButton_clicked` 0x4a7ed0, `guiUpdate` [**binario**] | **spenta** (`tune_timer`): Aurora non preme mai l'accordo |

### Scheda Notifiche e «gia' lavorato»

**Logica trovata nel programma** (`DisplayText::displayDecodedText`, 0x4eb170; `appendText`; `LogBook::match*`; `ADIF::match*`):

1. **Che cosa conta come «gia' lavorato»**: non c'e' un controllo a parte. Se **almeno una** delle sei evidenziazioni di «nuovo» e' accesa (`newCQZ`, `newITUZ`, `newDXCC`, `newGrid`, `newPx`, `newCall`; test del binario sui sei campi di `DisplayText` a +0x56, +0x59, +0x5c, +0x5f, +0x62, +0x65), JTDX parte dal presupposto «gia' lavorato» (sfondo del colore «Gia' lavorato» se `workedColor`, testo barrato se `workedStriked`, sottolineato se `workedUnderlined`) e poi controlla, **nell'ordine** zona CQ, zona ITU, paese, locatore, prefisso, nominativo: se per un criterio acceso la stazione e' nuova (prima in assoluto, poi per banda/modo) il messaggio prende il colore di quella novita' in grassetto, **perde barra e sottolineatura**, e il controllo si ferma (vince il primo). Cio' che resta e' «gia' lavorato». Senza nessuna evidenziazione accesa non c'e' nemmeno «gia' lavorato» e le opzioni sono disattivate.
2. **Il confronto col log**: `LogBook::matchCall/matchDXCC/matchGrid/matchPX/matchCQZ/matchITUZ(item, &worked, &workedBandMode, frequenza, modo)`. `worked` = l'elemento e' nel log in assoluto; se non lo e', `workedBandMode = false`. Altrimenti, se c'e' una banda o un modo, `workedBandMode = match(item, banda, modo)` con **banda e modo indipendenti** (vuoto = qualunque): casella «per banda» da sola = (banda, qualunque modo); «per modo» da sola = (qualunque banda, modo); **tutte e due = (banda, modo)**. La banda viene dalla frequenza di dial (`ADIF::bandFromFrequency`), il modo e' quello del programma (FT8 e FT4 sono modi diversi). Paese non riconosciuto = mai nuovo.
3. **Il log usato**: il file `wsjtx_log.adi` letto all'avvio (`ADIF::load`, filtrato da Generali) piu' i QSO registrati dopo (`LogBook::addAsWorked`). I suggerimenti di JTDX dicono proprio «...che non esistono nel file wsjtx_log.adi». In Aurora le sorgenti sono le stesse della scheda DXCC: log interno, `wsjtx_log.adi`/`aurora_log.adi`, Log4OM2, senza doppioni (`docs/scheda-dxcc.md`); un QSO appena registrato conta subito.
4. **Come si disegna** (`DisplayText::appendText`, [binario]): il testo della riga e' diviso a colonna 21 o 23 (`mid(4,1) == " "` → 21): la prima parte (ora, rapporto, DT, frequenza) resta com'e'; **sulla seconda parte, cioe' il messaggio, si applica il barrato** (proprieta' `FontStrikeOut` 0x2007) o la sottolineatura (stile 1; tratteggiata 2 o ondulata 6 per le DXpedition); poi si azzerano. Il colore di sfondo e' quello di tutta la riga.
5. **«non mostrare»** (`workedDontShow`): la riga si nasconde se e' una novita' «normale» non evidenziata (non forza il grassetto) e non e' un messaggio rivolto a me (`std_type != 2`); le stazioni **desiderate** riaccendono la riga.
6. **Inverti colore sfondo/testo** (`txtColor`): scambia sfondo e testo di tutti i messaggi (anche quelli «gia' lavorato»: la barra resta) tranne i messaggi tra altre stazioni quando «mostra potenziali» e' spento.
7. **Priorita' dei colori** (valori di `priority`, usati dalla sequenza): zona CQ 30/31, su banda 28/29, zona ITU 26/27 e 24/25, DXCC 22/23 e su banda 20/21, locatore 15/16 e 13/14, prefisso 11/12 e 9/10, nominativo 7/8 e 5/6; i desiderati 2-4 e 17-19. L'ordine in cui si controllano e' quello del punto 1.
8. **Altre voci**: `differentBackground` + `cyanBackground`/`yellowBackground`: i messaggi che contengono il mio nominativo hanno lo sfondo `#00ffff` (ciano) o `#ffff00` (giallo) [binario 0x4eb170, test su +0x8e/+0x8f/+0x90]; `highlightDXCall`, `highlightDXGrid`: evidenziano nei messaggi il nominativo e il locatore del DX; `redMarker`, `blueMarker`: cella rossa a fine riga sui messaggi rivolti a me, blu se e' della stazione in QSO; `OtherStandardMessagesMarker`: marcatore colorato sui messaggi tra altre stazioni con una novita'; `73RR73Marker`: i messaggi RR73/73 tra altre stazioni (stato `RFIN`) fanno entrare la stazione tra i candidati della sequenza; `newPotential`: le novita' valgono anche per i messaggi tra altre stazioni; `BeepOnFirstMsg`: un suono al primo messaggio di ogni periodo.

**Cosa fa Aurora**:

| Voce | Chiave | In Aurora |
|---|---|---|
| Evidenzia nuova zona CQ / ITU / paese / griglia / prefisso / nominativo, per banda, per modo | `new*`, `new*Band`, `new*BandMode` | **vive** (Applica): `WorkedLog::novelty_flags` (stesso ordine, stesse tre caselle, stessi argomenti banda/modo) alimenta le etichette delle decodifiche, `worked_call` compreso. Di fabbrica quelli di JTDX: paese e nominativo accesi con banda e modo, gli altri spenti. Il modo e' quello della radio (`FT8`, `FT4`); un QSO senza modo vale per tutti i modi; `DIG` raccoglie gli altri modi digitali (come nella scheda DXCC) |
| anche un suono | `BeepOnNew*` | i suoni per tipo di messaggio di Aurora (la riga accende il suono di «nuovo» e di «nuovo su banda/modo») |
| Evidenzia gia' lavorati (colore), barra, sottolinea, non mostrare | `workedColor`, `workedStriked`, `workedUnderlined`, `workedDontShow` | **vive** nell'elenco (`DecodeList`): barrato di fabbrica (come `workedStriked=true`), barra e sottolinea si escludono, tutte attive solo con almeno una evidenziazione di «nuovo»; «non mostrare» tiene i messaggi a me, le righe Tx e i desiderati. L'etichetta verde «GIA' LAVORATA» resta, ma neutra: il verde (riga e etichetta) solo con «evidenzia gia' lavorati», come in JTDX dove di fabbrica non c'e' colore |
| Inverti colore sfondo/testo | `txtColor` | **vivo**: il colore del tipo di messaggio va sul testo, lo sfondo resta neutro |
| Evidenzia altri messaggi predefiniti | `newPotential` | **vivo** (Applica) |
| Sfondo diverso per il mio nominativo (ciano o giallo), evidenzia i messaggi con il nominativo / il locatore DX, suono al primo messaggio | `differentBackground`, `cyanBackground`, `yellowBackground`, `highlightDXCall`, `highlightDXGrid`, `BeepOnFirstMsg` | **vivi** (browser). Il nominativo DX si evidenzia di fabbrica (come Aurora ha sempre fatto) |
| Marcatori | `OtherStandardMessagesMarker`, `redMarker`, `blueMarker`, `73RR73Marker` | **vivi**: altri messaggi (badge sui messaggi tra altre stazioni), rosso e blu (tacca a destra), 73/RR73 (Applica: `Options.rr73_marker`) |
| Nascondendo suggerimento | `hidehintMarker` | **spenta** (`no_hint`): il decodificatore di Aurora non produce «suggerimenti» |
| Avviso sul mio nominativo / sul nominativo DX, riproduci un suono di avviso (anche per banda) | `AlertOnMyCall`, `AlertOnDXCall`, `AlertOnNew*` e `OB` | **spente** (`alert_files`, `alert_dx`): JTDX riproduce un file audio scelto; Aurora ha i suoi suoni (Desiderati, Notifiche) |

Le scelte di disegno (`workedColor`... `highlightDXGrid`) stanno nel browser e si copiano sul server per dispositivo (chiave `aurora-notify-view`): non sono in `settings-motore.json`. Quelle che cambiano le etichette o la sequenza
(`new*`, `newPotential`, `73RR73Marker`) stanno nel file di Aurora e valgono per tutti i dispositivi.

**Differenze volute rispetto a JTDX**: JTDX conosce un solo log (`wsjtx_log.adi`); Aurora unisce le sorgenti. Il modo del QSO e' quello vero registrato (`FT8`, `FT4`); i QSO di altri modi digitali contano come `DIG`, CW e fonia contano
per paese e banda ma non per il modo dei decode. Senza modo noto (radio non letta) il livello banda/modo non si controlla, come in `LogBook::match*`.

## Da fare

1. **ALL.TXT**: collegare `AllTxt` a `EngineSettingsStore::reports().all_txt` (e al debug) e dichiarare `Feature::AllTxt` pronta; oggi l'unico interruttore e' `AURORA_ALL_TXT`.
2. **PSK Reporter** e **DXSummit**: formato e destinazione da rileggere in `jtdx.exe` con REA prima di scrivere qualsiasi invio; **eQSL** (credenziali da tenere fuori dal repository e dal file).
3. Opzioni del record ADIF (`toRTTY`, `dBtoComments`, `distanceToComments`), conferma al log (`PromptToLog`), spegnimento del log automatico, pulizia dei dati dopo il log.
4. Server TCP, filtri di testo sul flusso UDP, scelta dei dispositivi audio dal programma.
5. Nominativo e locatore a caldo (richiede un trasmettitore che li rilegga).
6. **Scheda Generali**: «La ricezione ritorna sull'ultima frequenza usata» (ricordare la frequenza all'ascolto e riscriverla alla radio), «Scorri la finestra dei messaggi in basso», «Nascondi i msg parzialmente trasmessi», prefisso al posto del nome del paese, allineamento di DXCC/distanza/azimut.
7. **Scheda Notifiche**: avvisi con un file audio scelto (`AlertOn*`), avviso sul nominativo DX; collegare l'interruttore `highlights` della sequenza (JTDX risponde a chiunque se nessuna evidenziazione di «nuovo» e' accesa) con le evidenziazioni, d'accordo con chi lavora sulla logica di QSO.
