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
| | `TxWatchdogTimer`, `TuneTimer` | Aurora ha i suoi tetti (Automazioni) e un watchdog fisso | nascoste, `tx_limits` |
| | `MonitorOFF`, `MonitorLastUsed` | Aurora ascolta sempre | nascoste, `own_monitor` |
| Radio | `Rig`, `PTTport`, `CATSerialPort`, `CATSerialRate`, `Polling`, `CATForceDTR/RTS`, `TCIAudio` | la radio e il PTT stanno nella scheda CAT | nascoste, `radio_cat` |
| Audio | `SoundInName`, `SoundOutName`, `AudioInputChannel`, `AudioOutputChannel` | dispositivi scelti all'avvio (`AURORA_AUDIO_DEVICE`, `AURORA_AUDIO_OUT_DEVICE`) | nascoste, `audio_start`; da fare la scelta dal programma |
| | `SaveDir`, `SaveWav` | registrazione dei periodi solo con `AURORA_NATIVE_RECORD` | nascoste, `no_wav_save` |
| Sequenza | `AutoSequence`, `SingleShotQSO`, `SkipTx1`, `AnswerWorkedB4`, `CallPriorityCQ`, `MaxDistance`, `StrictDirectionalCQ`, `Seq*` | la sequenza e le risposte sono le Regole di Aurora (pagina Automazioni) | nascoste, `own_rules` |
| Notifiche | `AlertOn*`, `BeepOn*` | avvisi di Aurora (Desiderati, Notifiche) | nascoste, `own_alerts` |
| Avanzate | `NDepth`, `NFT4Depth`, `NFT8Cycles`, `FT8Sensitivity`, ... | il decodificatore di Aurora ha parametri suoi | nascoste, `own_decoder` |
| Macro TX, Frequenze, Filtri, Programmazione | — | gia' di Aurora (non scrivevano nel `JTDX.ini`) | invariate |

## Da fare

1. **ALL.TXT**: collegare `AllTxt` a `EngineSettingsStore::reports().all_txt` (e al debug) e dichiarare `Feature::AllTxt` pronta; oggi l'unico interruttore e' `AURORA_ALL_TXT`.
2. **PSK Reporter** e **DXSummit**: formato e destinazione da rileggere in `jtdx.exe` con REA prima di scrivere qualsiasi invio; **eQSL** (credenziali da tenere fuori dal repository e dal file).
3. Opzioni del record ADIF (`toRTTY`, `dBtoComments`, `distanceToComments`), conferma al log (`PromptToLog`), spegnimento del log automatico, pulizia dei dati dopo il log.
4. Server TCP, filtri di testo sul flusso UDP, scelta dei dispositivi audio dal programma.
5. Nominativo e locatore a caldo (richiede un trasmettitore che li rilegga).
