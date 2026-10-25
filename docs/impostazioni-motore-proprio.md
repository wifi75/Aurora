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
| | Spotting DXSummit (`AllowSpotsDXSummit`) | permette lo spot verso DXSummit, che parte solo dal pulsante «DX Call». [ini] (letto in `jtdx.exe`: `on_pbSpotDXCall_clicked`) | pulsante «Spot DX» e rotta `POST /api/v1/spot/dxsummit` (sezione «Spot su DXSummit» sotto) | **viva**, **accesa di base** |
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
| | `Decode52`, `After73`, `IDint`, `TuneTimer`, `MonitorLastUsed`, `Scroll`, `HideTxMessages`, `countryPrefix`, `AlignDistanceAzimuth` | visibili e spente con il motivo (vedi sotto) | spente |
| | `InsertBlank`, `Tx2QSO`, `showAzimuth` | opzioni dello schermo, nel browser (valgono subito) | **funzionano** |
| Radio | `Rig`, `PTTport`, `CATSerialPort`, `CATSerialRate`, `Polling`, `CATForceDTR/RTS`, `TCIAudio` | la radio e il PTT stanno nella scheda CAT | nascoste, `radio_cat` |
| Audio | `SoundInName`, `SoundOutName`, `AudioInputChannel`, `AudioOutputChannel`, `SaveDir`, `SaveWav` | ingresso e uscita scelti nella scheda Audio; senza scelta valgono `AURORA_AUDIO_DEVICE` e `AURORA_AUDIO_OUT_DEVICE` | vive, a caldo; si cambiano solo da `PUT /api/v1/audio/settings` (apre i dispositivi, poi salva) |
| | `SaveDir`, `SaveWav` | registrazione dei periodi solo con `AURORA_NATIVE_RECORD` | nascoste, `no_wav_save` |
| Sequenza | `Seq*Count`, `Seq*CounterValue` (quattro scenari), `SeqHaltTxReplyOther`, `StrictDirectionalCQ` | **scheda Sequenza come in JTDX**: vedi «Scheda Sequenza» sotto | **funzionano** (a caldo) |
| | `CallPriorityCQ`, `MaxDistance` | «Priorita' ai CQ» e «Priorita' alla distanza maggiore»: `Options.call_prio_cq` e `max_distance` | **funzionano** (a caldo) |
| | `AutoSequence`, `SingleShotQSO`, `SkipTx1`, `AnswerWorkedB4` | **gli stessi interruttori del pannello QSO** (`autoseq`, `single_qso`, `skip_tx1`, `answer_b4`, ricordati in `controls.json`): la scheda li mostra e li cambia con `GET /controls` e `POST /controls/{id}`, nessuna copia nel file delle impostazioni | **funzionano** (stato unico); nel documento API restano `own_rules` |
| Notifiche | `newCQZ/ITUZ/DXCC/Grid/Px/Call` (+ `Band`, `BandMode`), `newPotential`, `73RR73Marker` | **scheda Notifiche come in JTDX**: vedi «Scheda Notifiche» sotto | **funzionano** (a caldo) |
| | `workedColor`, `workedStriked`, `workedUnderlined`, `workedDontShow`, `txtColor`, `OtherStandardMessagesMarker`, `redMarker`, `blueMarker`, `differentBackground`, `highlightDXCall`... | come si disegnano i messaggi: scelte del browser, per dispositivo, valgono subito | nascoste, `own_notify` (nel documento API); **funzionano** nell'interfaccia |
| | `BeepOn*` | «anche un suono»: i suoni per tipo di messaggio di Aurora | **funzionano** (nel browser) |
| | `AlertOnMyCall`, `AlertOnDXCall`, `AlertOnNewCQZ/ITUZ/DXCC/Grid/Px` e `...OB` («Play alert sound» e «per banda») | l'avviso con il suono scelto: **vivi** nel browser (`alertMyCall`... in `aurora-notify-view`), vedi «Avvisi sonori» sotto | **funzionano**; nel documento API restano `own_alerts` |
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

### Regione IARU e tabella delle frequenze (2026-10-25)

La tabella delle frequenze (`frequencies.json`, scheda Frequenze, `GET/PUT /api/v1/frequencies`) ha ora la tabella iniziale di JTDX per FT8 e FT4 (41 righe, 160 m-2 m) con, per ogni riga, la **regione IARU** (`ALL`, `R1`, `R2`, `R3`) e se e' «predefinita» (`preferred`). La voce `Region` di Generali (valori `ALL`, `R1`, `R2`, `R3`, a caldo, valore iniziale dal `JTDX.ini` o `ALL`) decide quali righe valgono:

- una riga si vede con `ALL`, se e' valida ovunque o se e' della regione scelta (stesso filtro di JTDX); le righe nascoste restano nella tabella e nella scheda, attenuate;
- cambio di banda e programmazione (`FrequencyTable::frequency_in`): riga predefinita, poi quella della regione scelta (con `ALL` una valida ovunque), poi la frequenza piu' bassa. Con `ALL` e `R2` ogni banda ha il valore di sempre; con `R1` i 160 m vanno su 1,810 MHz e con `R3` gli 80 m in FT4 su 3,568 MHz;
- cambio di modo (`FrequencyTable::best_working`): come `switch_mode` di JTDX, resta sul dial se e' una riga del nuovo modo, altrimenti la riga predefinita piu' vicina della banda;
- `GET /api/v1/frequencies/defaults` restituisce la tabella iniziale (tasto «Frequenze iniziali di JTDX»: la mette nella bozza, si salva con «Salva»);
- una tabella salvata dalle versioni precedenti (una riga per banda e modo, uguale alla vecchia tabella di Aurora) passa da sola alla tabella di JTDX; una tabella modificata dall'utente non si tocca.

Resta fuori: le righe di 8 m (40,680 MHz), 4 m (70 MHz) e 70 cm (Aurora non ha quelle bande) e le alternative non predefinite non si scelgono da una tendina.

### Scheda Generali

| Voce | Chiave | Funzione che la legge | In Aurora |
|---|---|---|---|
| Mio nominativo, Mio locatore | `MyCall`, `MyGrid` | `Configuration::read_settings` [GPL 2169-2170] | **vivi**, dal prossimo avvio |
| Regione IARU | `Region` ([ini] `@Variant(IARURegions::Region)`: `ALL`, `R1`, `R2`, `R3`; fabbrica `ALL`) | `switch_mode` `REA 0x47b130` -> `FrequencyList_v2::filter` `REA 0x5a61d0` / `filterAcceptsRow` `REA 0x654aa0`, `best_working_frequency` `REA 0x654ca0` [**binario**]: filtra le righe della tabella delle frequenze (sezione «Regione IARU» di `docs/analisi-jtdx-reverse-rea.md`) | **viva**, a caldo: `FrequencyTable::frequency_in` / `best_working` (cambio di banda, di modo, programmazione) e righe attenuate nella scheda Frequenze |
| Generazione dei messaggi per nominativi composti di tipo 2 | `Type2MsgGen` ([ini] `@Variant`: `type_2_msg_1_full`, `type_2_msg_3_full` (fabbrica), `type_2_msg_5_only`) | un solo chiamante, `genStdMsgs` `REA 0x474800`, **solo nel ramo dei modi che non cominciano per «FT»** (JT65, JT9, T10) [**binario**] | **viva come voce** (salvata, importata dal `JTDX.ini`, `Reports.type2_msg_gen`) ma **non cambia mai niente** in FT8 e FT4, come in JTDX: la pagina lo scrive sotto la voce (`note: jt_only`). I nominativi composti di FT8/FT4 sono in `qso-native::stdmsg` |
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
| Nascondendo suggerimento | `hidehintMarker` | **spenta** (`no_hint`): il decodificatore di Aurora non produce «suggerimenti». E' l'**unica** voce della scheda Notifiche che resta spenta |
| Avviso sul mio nominativo / sul nominativo DX, «Play alert sound» per zona CQ, zona ITU, paese, griglia, prefisso, e «per banda» | `AlertOnMyCall`, `AlertOnDXCall`, `AlertOnNewCQZ/ITUZ/DXCC/Grid/Px` e `OB` | **vivi**: un messaggio nuovo (non nostro) che chiama me, nomina il DX scelto o porta quell'etichetta fa suonare il suono scelto; «per banda» = l'etichetta «nuovo su banda/modo» di quel tipo (la stessa che decide `new*Band`/`new*BandMode`). Il «per banda» vale solo con l'avviso del suo tipo |

**Avvisi sonori** (scheda Notifiche): vedi «Suoni per funzione» sotto. Restano il pulsante «Avvisi sonori» (acceso/spento e volume), «Non suonare mentre trasmetti» (di fabbrica acceso), i due canali «anche un suono» (`BeepOn*`) e «Play alert sound» (`AlertOn*`), «Suono al primo messaggio» e «Suono per il mio». I messaggi gia' visti, le righe Tx e quelli arrivati prima di aprire la pagina non suonano. Il browser chiede un tocco sulla pagina prima di far suonare qualcosa (regola di WebAudio).

Le scelte di disegno (`workedColor`... `highlightDXGrid`) stanno nel browser e si copiano sul server per dispositivo (chiave `aurora-notify-view`): non sono in `settings-motore.json`. Quelle che cambiano le etichette o la sequenza
(`new*`, `newPotential`, `73RR73Marker`) stanno nel file di Aurora e valgono per tutti i dispositivi.

**Differenze volute rispetto a JTDX**: JTDX conosce un solo log (`wsjtx_log.adi`); Aurora unisce le sorgenti. Il modo del QSO e' quello vero registrato (`FT8`, `FT4`); i QSO di altri modi digitali contano come `DIG`, CW e fonia contano
per paese e banda ma non per il modo dei decode. Senza modo noto (radio non letta) il livello banda/modo non si controlla, come in `LogBook::match*`.

### Suoni per funzione (2026-10-25)

- **Banco dei suoni** (`app/src/lib/soundBank.ts`): 53 suoni sintetizzati con WebAudio, nessun file e nessuna licenza, in sei categorie (campane e campanelli, bip e toni, scale e melodie, sirene e segnali, radio e telegrafo, gocce ed effetti), tra cui i quattro di prima (campanelle, campana, pip pip, tromba) e le note standard di prima (tre note in salita, nota singola, trillo, quattro note). Ogni suono e' un dato: note con frequenza, inizio, durata, forma d'onda, inviluppo (`flat`, `fade`, `pluck`, `swell`), glissando, vibrato e un filtro facoltativo; durano da 0,2 a 1,5 s, partono e finiscono a zero (niente scatti) e il volume e' normalizzato (peso per forma d'onda e divisione per le note simultanee). Nomi e categorie nelle sei lingue. `playSound(id, volume)`, `previewSound` (pulsante «Prova»: riprende il contesto se sospeso), `stopAll()`, un solo contesto audio riusato e ripreso al primo gesto; sospeso, non mette note in coda.
- **Assegnazione** (`app/src/lib/soundAssign.ts`, chiave `aurora-sound-assign`, copiata sul server come le altre `aurora-*`): una mappa funzione -> suono o nessuno per le 17 voci con colore (mio messaggio in trasmissione, CQ, chiamata a me, stazione comune, nuova zona CQ/ITU, nuovo DXCC, griglia, prefisso, stazione, ognuna anche «su nuova banda», gia' lavorata) e per gli avvisi di prima (avviso chiamata verso di me, avviso nominativo DX scelto, primo messaggio decodificato, stazione desiderata).
- **Quando suona**: serve «Avvisi sonori» acceso, l'avviso di quella funzione acceso come prima («anche un suono», «Riproduci un suono di avviso», «Suono al primo messaggio») **e** un suono assegnato. Senza suono assegnato tace. Scegliere un suono accende l'avviso della funzione; accendere un avviso senza suono gli da' il suono di prima. Se piu' funzioni scattano nello stesso gruppo suona la piu' importante (chiamata a me, avvisi, nuovo paese, zone, prefissi, griglie, nominativi, comuni, CQ, gia' lavorate). «Mio messaggio in trasmissione» suona anche con «non suonare mentre trasmetti» (e' il segnale del Tx); tutte le altre funzioni tacciono mentre si trasmette.
- **Migrazione**: senza assegnazione salvata, chi aveva un suono acceso per un tipo ottiene il suono standard di prima (chiamata a me: trillo; nuovo paese: tre note; le altre novita': nota singola; stazione desiderata: quattro note) e gli avvisi «Play alert sound» il suono che avevano scelto; chi non aveva mai toccato niente non ha nessun suono. Se per una novita' erano accesi sia «anche un suono» sia l'avviso, resta solo il suono scelto (prima suonavano due cose insieme).
- **Interfaccia**: in ogni riga dell'anteprima «Colori e anteprima» un menu «Suono» compatto (Nessuno, poi i suoni per categoria) e il pulsante ▶, al posto della nota ♪; in fondo alla scheda il riquadro «Suoni» (volume generale, «Assegna suoni diversi automaticamente», «Togli tutti i suoni», «Ferma», le quattro funzioni che non sono un tipo di messaggio con il loro stato, e il banco intero da provare). Nessun suono parte aprendo la scheda. Anche con JTDX la tabella dei tipi ha il menu accanto alla casella del suono. Il vecchio menu unico «Suono dell'avviso» e il suo «Prova» sono stati tolti: il suono si sceglie per funzione.
- **Resta da fare**: l'ascolto sul campo (volumi relativi dei suoni a confronto con la radio in cuffia); un suono diverso per ogni funzione e' una scelta dell'utente («Assegna suoni diversi» ne propone una); le funzioni dei messaggi ricevuti si calcolano nel browser (come prima), non nel core.

## Da fare

1. **ALL.TXT**: collegare `AllTxt` a `EngineSettingsStore::reports().all_txt` (e al debug) e dichiarare `Feature::AllTxt` pronta; oggi l'unico interruttore e' `AURORA_ALL_TXT`.
2. **eQSL**: credenziali tenute fuori dal repository e dal file (protezione dei segreti del sistema, gia' fatto); PSK Reporter e DXSummit sono fatti (sezione «Scheda Segnalazioni» in fondo).
3. Opzioni del record ADIF (`toRTTY`, `dBtoComments`, `distanceToComments`), conferma al log (`PromptToLog`), spegnimento del log automatico, pulizia dei dati dopo il log.
4. Server TCP, filtri di testo sul flusso UDP.
5. Nominativo e locatore a caldo (richiede un trasmettitore che li rilegga).
6. **Scheda Generali**: «La ricezione ritorna sull'ultima frequenza usata» (ricordare la frequenza all'ascolto e riscriverla alla radio), «Scorri la finestra dei messaggi in basso», «Nascondi i msg parzialmente trasmessi», prefisso al posto del nome del paese, allineamento di DXCC/distanza/azimut.
7. **Scheda Notifiche**: avvisi con un file audio scelto (`AlertOn*`), avviso sul nominativo DX; collegare l'interruttore `highlights` della sequenza (JTDX risponde a chiunque se nessuna evidenziazione di «nuovo» e' accesa) con le evidenziazioni, d'accordo con chi lavora sulla logica di QSO.

## Scheda Audio: collegata (2026-10-25)

Logica di JTDX letta con REA in `jtdx.exe` 2.2.159 (`SoundInput::start` 0x4e27e0, `Detector::writeData` 0x62f680, `Modulator::readData` 0x63c480, `Configuration::impl::update_audio_channels` 0x53ce70,
`MainWindow::dataSink` 0x4bfae0, `killFile` 0x4c22d0, `BWFFile::impl::update_header` 0x61a620) e nei sorgenti:
- **Ingresso**: Mono apre il dispositivo con un canale (somma del mixer), Sinistro/Destro lo aprono in stereo e tengono il primo/secondo campione; Entrambi in ingresso non c'e' (vale Sinistro).
- **Uscita**: Mono un canale; Sinistro `[onda, 0]`; Destro `[0, onda]`; Entrambi `[onda, onda]`. Voci abilitate solo se il dispositivo ha i canali (Mono con 1, Sinistro/Destro con 2; Entrambi solo in uscita): la scheda lo rispetta e riporta a Mono se si sceglie un dispositivo con un solo canale.
- **File audio**: `SaveWav` 0 Nessuno, 1 Decodificati, 2 Tutti; nome `aammgg_hhmmss.wav` (UTC, inizio periodo) in `SaveDir` (predefinita `<dati>/save`); a fine decodifica il file si cancella salvo Tutti o Decodificati con almeno un messaggio; WAV 12 kHz mono 16 bit con blocco `LIST/INFO` (ISRC, ISFT, ICRD, ICMT).

**Come e' collegata** (`core/crates/aurora/src/audio_control.rs`, `remote-api/src/audio_api.rs`, `app/src/settings/AudioDevicesPanel.tsx`):
- **Rotte**: `GET /api/v1/audio/devices` (ingressi e uscite con canali e predefinito, dispositivi in uso, valori salvati) e `PUT /api/v1/audio/settings` (solo rete di casa): la rotta controlla i valori, chiede al gestore di applicarli e **salva nello store solo le parti riuscite**; se la scrittura del file fallisce i dispositivi tornano come erano. Dettagli in `docs/api.md`.
- **Valore iniziale** (nessun cambio per chi non tocca niente): ingresso = lo store (importato per chiave da `JTDX.ini`) e, se e' vuoto, `AURORA_AUDIO_DEVICE`; con tutti e due vuoti la cascata resta spenta come prima. Uscita: lo store, poi `AURORA_AUDIO_OUT_DEVICE`, poi il predefinito del sistema. Un nome dello store che non si apre ripiega sull'ambiente, **mai** sul predefinito del sistema (per non trasmettere su un'uscita sbagliata: senza ripiego niente trasmissione, come prima con un nome sbagliato). `Default` (come nel `JTDX.ini`) vuol dire «predefinito del sistema»; per l'uscita vuoto e `Default` sono la stessa scelta.
- **Ingresso a caldo**: il gestore ferma la cattura, apre quella nuova (stessa coda verso il decodificatore, quindi nessuna perdita del motore: il ricampionamento riparte da solo se cambia la frequenza della scheda) e fa seguire il livello d'ingresso (`InputLevelService::set_control`, volume di registrazione di Windows del dispositivo nuovo). Se il nuovo non si apre **si riapre il vecchio** con il suo canale e la scelta non si salva. Il decodificatore parte alla prima apertura di un ingresso: con l'ingresso mai aperto non c'e' niente da decodificare, come prima. Svuotare l'ingresso con la ricezione accesa e' rifiutato (non spegne niente).
- **Uscita a caldo**: la trasmissione usa un `SwitchableSink`; l'uscita nuova si apre prima e subentra a quella vecchia tra una trasmissione e l'altra (`deferred` se se ne sta facendo una); se non si apre la vecchia non si tocca. Il canale d'uscita vale subito. Se all'avvio la catena di trasmissione non e' partita (uscita o PTT non aperti) la scelta dell'uscita si salva e vale dal prossimo avvio (`saved_only`).
- **File audio**: `SaveSettings` condiviso con il motore (`start_with_save`), a caldo dal periodo dopo; la cartella si crea e si prova subito e un percorso inutilizzabile e' rifiutato (con «Nessuno» non si controlla).
- **Interfaccia**: scheda Impostazioni → Audio con menu a tendina per ingresso e uscita («Predefinito del sistema (nome)», canali, dispositivo in uso evidenziato, «Aggiorna elenco»), canali (Sinistro/Destro/Entrambi spenti sui dispositivi mono), modalita' e cartella dei file; sotto, l'esito per parte (applicato, rimandato alla prossima trasmissione, non applicato con il motivo e il dispositivo che resta in uso). Testi e suggerimenti nelle sei lingue.
- Prove: `remote-api/tests/audio.rs` (rotte con gestore finto, ripristino, persistenza), `aurora::audio_control::tests` (dispositivi finti: valori iniziali, ripiego, ripristino dell'ingresso, cambio di uscita differito, file audio), `input-level` (segue il dispositivo), `app/src/settings/AudioDevicesPanel.test.tsx`.

**Resta**: «Usa TCI Audio» (TCI non c'e') e la memoria di potenza per banda/modo (Tx, Tune) della scheda di JTDX; il livello d'ingresso (`/input-level`) esiste solo se all'avvio c'era un ingresso da regolare (se la cascata e' nata dalla scheda Audio, il controllo compare al riavvio successivo); il canale d'uscita cambiato a meta' trasmissione vale subito; prova con dispositivi reali (radio, USB) a cura del proprietario.

## Scheda Segnalazioni: stato dopo il lavoro del 2026-10-25 (sostituisce le righe «spente» della tabella sopra)

Evidenza: chiavi lette da `Configuration::impl::read_settings` di `jtdx.exe` 2.2.159 (xref di ogni stringa) e dai sorgenti pubblici; funzioni lette in `LogQSO::initLogQSO` 0x60aa90, `LogQSO::accept` 0x60be60, `MainWindow::acceptQSO2` 0x472db0, `logClearDX` 0x4774b0, `on_pbSpotDXCall_clicked` 0x4abdb0, `postDecode` 0x477540, `EQSL::upload` 0x5f7650, `PSK_Reporter` (modelli IPFIX a 0x13bea58 e 0x13beab8, timer 300000 ms, porta 4739, limite 1400 byte). Il binario coincide con i sorgenti.

| Voce | Stato | Cosa fa / come si prova |
|---|---|---|
| `toRTTY`, `dBtoComments`, `distanceToComments` | **vive** | `logbook::adif::log_comment` e `log_mode` (test); commento `FT8  Sent: -10  Rcvd: -12  Distance: 1234 km` nel campo `COMMENT` di UDP, file, TCP, QsoLogged UDP. `toRTTY` in JTDX vale solo per JT9: in FT8/FT4 non cambia niente (nota sotto la voce). Distanza: cerchio massimo, puo' differire di qualche km da `azdist` |
| `ClearCallGrid` | **viva** | `autoStopTx` (`clear_dx` o autolog) e timer di fine periodo `logClearDXTimer` solo senza AutoSeq/AutoTX (test `engine-native`) |
| `ClearCallGridExit` | **spenta** (`clear_exit`) | Aurora riparte sempre senza DX |
| `PromptToLog`, `AutoQSOLogging` | **spente** | manca la finestra di conferma/registrazione a mano nella pagina (eventi `QsoCompleted`/`QsoPending` e `PendingQso` gia' nel core); il QSO si registra sempre da solo |
| `TCPServer`, `TCPServerPort`, `EnableTCPConnection` | **vive** | un messaggio `<command:3>Log <parameters:N> ...` per QSO, attesa 1 s, `NAK` = errore (`reporting::tcp`, test contro server locale) |
| `PSKReporter` | **viva** | IPFIX identico a JTDX verso `report.pskreporter.info:4739` ogni 5 minuti (`reporting::psk`, test con socket UDP locale). Solo messaggi standard con nominativo e locatore validi, non «ipotizzati» (`hint`), non del periodo del cambio banda. Antenna vuota; programma `Aurora vX` |
| `AllowSpotsDXSummit` | **viva**, di fabbrica **accesa** (scelta del proprietario; con un `JTDX.ini` vale il suo valore) | pulsante «Spot DX» e rotta `POST /api/v1/spot/dxsummit` (sezione «Spot su DXSummit» sotto); l'ultimo invio sta accanto alla voce (`status.dxsummit`) |
| `EQSLSend`, `EQSLUser`, `EQSLNick`, `EQSLPasswd` | **vive** (con DPAPI) | ADIF di `EQSL::upload` su **https** (JTDX usa http); la password sta in `eqsl.secret` cifrato con DPAPI, mai nel JSON ne' nelle risposte |
| `ApplyFiltersToUDPmessages` | **spenta** (`udp_filters`) | `aurora_core::UdpFilters` e l'emettitore (`apply_filters`) sono pronti e provati; mancano la copia dei filtri dalla pagina e la rotta |
| `preventFalseUDPspots` | **viva con nota** | il marcatore `?` esiste solo per JT65/JT9: in FT8/FT4 non esclude niente, come in JTDX. Il bit di bassa fiducia UDP segue ora `hint` |
| `udpWindowToFront`, `udpWindowRestore` | **spente** (`udp_window`) | l'emettitore chiama `on_reply_accepted` (test); manca il collegamento con la finestra Tauri |
| `WriteDecodedALLTXT`, `...DebugALLTXT` | **vive** | `AllTxtScope`: senza caselle solo i messaggi con il proprio nominativo; debug aggiunge «Decoding finished». Le due caselle si escludono |

Tutti gli invii verso internet partono in un'attivita' a parte con tempo massimo, si spengono dall'interruttore e riportano l'ultimo esito (`status` in `GET /api/v1/engine-settings`).

### Spot su DXSummit

Cosa fa `MainWindow::on_pbSpotDXCall_clicked` in `jtdx.exe` 2.2.159 (letto nell'eseguibile; coincide con i sorgenti):

- **Quando**: solo dal pulsante «DX Call», mai da una decodifica. Non fa niente se lo spotting e' spento (`spot_to_dxsummit`), se la stazione e' gia' stata segnalata (il flag si azzera a ogni cambio del nominativo DX e con «Canc DX»), se il nominativo DX e' vuoto o se manca il proprio.
- **Cosa invia**: `GET http://www.dxsummit.fi/SendSpot.aspx` con `callSign` (proprio nominativo), `dxCallSign`, `frequency` e `info`.
- **Frequenza**: (frequenza della radio + valore di **Rx** in Hz) / 1000, in kHz con **un decimale**. E' la frequenza di ricezione (`RxFreqSpinBox`: dove si sente il DX, che JTDX sposta sulla stazione scelta), non quella di trasmissione; Aurora fa lo stesso (radio + `rx_df`).
- **Commento (`info`)**: l'anteprima della riga di testo dello spot senza il prefisso «info: » (`on_spotLineEdit_textChanged`): il modo (`FT8`), poi la propagazione (campo `propLineEdit`, solo se il testo non ha `#G`), poi il testo con le macro espanse: `#G` = `proprioLocatore<propagazione>locatoreDX`, `#D` = distanza senza spazi (`1234km`), `#R` = rapporto (`-12dB`), `#H` = `F/H` solo in Fox/Hound (in Aurora sparisce). Senza testo ne' propagazione resta il solo modo; con un testo e senza propagazione restano, come nel sorgente, due spazi dopo il modo.
- **Attesa**: 10 secondi. Se scade o la connessione fallisce: finestra «Can not establish/complete connection to dxsummit server», il flag non si imposta e si puo' riprovare. Se va bene il pulsante diventa «Spotted» fino al prossimo cambio di DX.

Aurora: `reporting::dxsummit` (campi, frequenza, `compose_info`); `reporting::Service::spot_current` / `spot_dxsummit` (frequenza e modo li legge dal motore; guardia: la stessa stazione non due volte in 5 minuti, un invio fallito non conta; invio in un thread a parte con attesa massima di 10 s, il motore non si ferma mai); la rotta `POST /api/v1/spot/dxsummit` (solo rete di casa, vedi `docs/api.md`). Nella pagina Stazione, riquadro Trasmissione, riga DX: il pulsante «Spot DX» (con la casella «Commento spot») compare solo se lo spotting e' acceso e c'e' una stazione DX; dopo l'invio dice «Spottato» finche' resta quella stazione, e l'esito in chiaro (inviato o errore) compare sotto i comandi. L'esito dell'ultimo invio sta anche nella scheda Segnalazioni, accanto alla voce.

Prove: `reporting` (unit e `tests/service.rs`: server HTTP locale, campi, radio + Rx, stessa stazione due volte, errore 500, server che non risponde), `remote-api/tests/spot.rs` (401, 403 da fuori, 404 senza servizio, 200, 409, 400, 502), `app/src/components/TxCommands.test.tsx`, `app/src/settings/NativeSettings.test.tsx`.
