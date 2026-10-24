# Fase 0 — Rapporto (in compilazione)

## Esito delle ipotesi

| # | Ipotesi | Stato | Prova |
|---|---|---|---|
| H1 | JTDX 159 parla il protocollo UDP di WSJT-X | **CONFERMATA** da 64 pacchetti reali (2026-10-24): tutti decodificati da `udp-bridge`, 0 errori | prova manuale 1 (`fase0-prove-manuali.md`) |
| H2 | Cattura audio in parallelo a JTDX | da verificare | prova 3; unico ingresso visto: `Microfono (2- USB Audio Device)` |
| H3 | Lettura/scrittura frequenza e PTT | da verificare | prova 4; `rigctld-jtdx` supporta `-m -r -s -t` e le porte TCP (aiuto verificato) |
| H4 | JTDX stabile minimizzato | da verificare | prova 5 |
| H5 | `Reply` UDP avvia un QSO completo | **CONFERMATA dalla prova (2026-10-24)**: il Reply ha compilato DX Call e locatore, **acceso Enable Tx e fatto trasmettere**. **supportata dal reverse** (serve l'opzione "Accept UDP requests"; avvio del QSO da confermare con la prova 2) | prova 2 |

## Decisioni prese durante l'esecuzione

- **Task 1:** con `members = ["crates/*"]` e nessun crate, `cargo metadata` fallisce su Windows (il glob non trova corrispondenze). La verifica si fa dopo il Task 2, quando esiste il primo crate.
- **Task 3:** `decode` restituisce `Packet { schema, message }` invece di `Message` e non esiste il tipo `Header`: lo schema serve ai comandi del Task 4 e l'identificativo sta già in ogni messaggio.
- **Task 3:** niente `Error::Unsupported`: lo schema non cambia il layout dei messaggi modellati, e i tipi sconosciuti producono `Message::Unknown` con i byte originali.
- **Task 3:** i campi finali di `Status` (da `special_op_mode` in poi) sono opzionali, perché versioni o schemi più vecchi di JTDX possono non averli; il test di troncamento richiede l'errore solo prima della fine della parte obbligatoria.
- **Task 3:** il test sulle catture reali è `#[ignore]` con motivo esplicito finché `tests/fixtures/` è vuota; va riabilitato dopo la cattura.

## Differenze tra pacchetti reali e il formato ipotizzato

(da compilare dopo la cattura)

## Lavoro automatico completato

- `udp-bridge`: decodifica di Heartbeat, Status, Decode, Clear, QsoLogged, Close e tipi sconosciuti; codifica di HaltTx, FreeText, Reply. 20 test automatici verdi (sintetici); il test sulle catture reali è in attesa dei pacchetti.
- `probes`: `udp_capture`, `udp_send` (verificato in locale: il pacchetto di HaltTx arriva con magic, schema e tipo corretti), `audio_tone` (picco di frequenza, 3 test).
- Inventario delle funzioni: bozza in `fase0-parita-funzioni.md`.
- Sull'`API` di `cpal 0.18.2`: differisce dalle versioni precedenti (`description()` al posto di `name()`, configurazione passata per valore, errore `cpal::Error`); il probe è già adattato.

## Reverse di `jtdx.exe` con REA + Ghidra (eseguito il 2026-10-24)

Strumenti: REA 6.7.0 (`rea`), Ghidra 12.1.4 (zip ufficiale, hash SHA-256 verificato), JDK 25. L'analisi di `jtdx.exe`
(PE x86-64, base 0x400000, 18 segmenti, SHA-256 `5cce71a6…182b`) ha richiesto 283 secondi e ha trovato 6143 procedure e 8704 stringhe.

Risultati dalla ricerca delle stringhe (123 corrispondenze su UDP, Reply, MessageClient, Halt):

- **Server UDP primario e secondario**: campi "UDP Server" / "UDP Server port number" (porta 0 = nessun invio) e "2nd UDP server"; errori "UDP server blocked, please try another" e "UDP server lookup failed", classe `MessageClient`. Conferma che JTDX espone lo stesso schema di WSJT-X.
- **Reply da UDP**: log "UDP Reply request received", "processed", "ignored: UDP accept option disabled", "ignored, decode not found", "rejecting UDP request to reply as decode is not valid". Il Reply cerca un decode noto: si deve rimandare il decode originale, come fa `encode_reply`.
- **Opzioni da attivare in JTDX**: Impostazioni → Reporting → "Accept UDP requests"; menu "Accept UDP Reply messages" con tre modi: solo CQ (`actionAcceptUDPCQ`), CQ e 73 (`actionAcceptUDPCQ73`), qualsiasi messaggio (`actionAcceptUDPAny`).
- **Effetti collaterali**: "Accepted UDP request restores window" e "Notify on accepted UDP request" (JTDX può riportare in primo piano la sua finestra); impatta H4.
- **Log via UDP**: invio del QSO in formato ADIF ai server UDP (`EnableUDP1adifSending`, `EnableUDP2adifBroadcast`): si può usare per il log di Aurora.
- **Filtri sui messaggi UDP**: "Prevent spotting messages with the unconfirmed callsigns via UDP" e "Apply text filters to transmission of the UDP messages": alcuni decode possono non arrivare ad Aurora.
- Altri controlli radio trovati: Ham Radio Deluxe, DX Lab Suite Commander, TCI.

Prossimo passo con REA: trovare la funzione che costruisce e legge i messaggi (xrefs delle stringhe) per confermare l'ordine dei campi.

### Gestore del Reply: `MainWindow::replyToUDP` (decompilato da REA, funzione a 0x4778ab)

Firma recuperata: `replyToUDP(QTime, int, float, unsigned int, QString const&, QString const&, bool, unsigned char)`,
cioe' **ora, SNR, delta tempo, delta frequenza, modo, messaggio, bassa confidenza, modificatori**: lo stesso ordine di `encode_reply`.

Cosa fa, nell'ordine:

1. Se il log di debug e' attivo scrive in ALL.TXT "UDP Reply request received: ...". Se "Accept UDP requests" e' spento: "ignored: UDP accept option disabled" e termina.
2. **Filtro per il modo scelto nel menu "Accept UDP Reply messages"**:
   - 1 = solo CQ: il messaggio deve iniziare con `CQ `, `CQDX ` o `QRZ`;
   - 2 = CQ e 73: come sopra, oppure testo che finisce con ` 73` o ` RR73`;
   - 3 = qualsiasi messaggio.
   Se non passa: "rejecting UDP request to reply as decode is not valid".
3. **Ritrova il decode nel testo della finestra**: ricostruisce la riga `hhmmss SNR(3) DT(4, 1 decimale) DF(4) modo messaggio` (per i modi lenti l'ora e' `hhmm`) e la cerca con `lastIndexOf`; se manca prova anche la variante con il segno del DT; se non c'e': "decode not found". **Quindi il Reply funziona solo su un decode ancora visibile in JTDX e identico in ora, SNR, DT (a 1 decimale), DF, modo e testo.**
4. Se trovato: invia a sua volta uno Status UDP (`MessageClient::status_update`), poi, secondo le opzioni, porta la finestra in primo piano ("udpWindowToFront") o la ripristina se minimizzata ("udpWindowRestore").
5. Chiama **`processMessage` con un flag di "richiesta UDP"**, cioe' la stessa elaborazione del doppio clic sul decode, poi `txwatchdog(false)` e, se previsto, l'avviso dell'applicazione. Se poi parta davvero la trasmissione dipende dalle impostazioni di JTDX (Enable Tx, sequenza automatica).

Conseguenze per Aurora:
- Mantenere il decode **originale** ricevuto (ora, SNR, DT, DF, modo, testo) e rimandarlo senza modifiche; rispondere finche' il decode e' ancora nella finestra di JTDX (stesso periodo o quello successivo, prima di cancellare l'attivita' di banda).
- Nelle impostazioni di JTDX: attivare "Accept UDP requests", scegliere il modo di accettazione e **disattivare "porta in primo piano" e "ripristina la finestra"** per non farsi togliere il fuoco a ogni risposta.
- Per verificare le risposte basta attivare il log di debug di JTDX e leggere ALL.TXT: contiene "UDP Reply request received / processed / ignored".
- Resta da confermare (decompilando i costruttori dei messaggi) la larghezza dei campi sul filo (`float` o `double`).

### Formato dei messaggi UDP confermato dal reverse (REA su `jtdx.exe`)

Funzioni decompilate: `NetworkMessage::Builder::Builder` (0x5967b0), `MessageClient::decode` (0x56e420), `impl::heartbeat` (0x56dd80),
`impl::parse_message` (0x56c360), `qso_logged` (0x56a164), `WSPR_decode` (0x56a5e4). Elenco completo delle funzioni `MessageClient::` e `NetworkMessage::` ottenuto con `rea search --kind procedures`.

**Intestazione** (identica a WSJT-X): numero magico `0xADBCCBDA` (u32), schema (u32), tipo (u32), identificativo (stringa Qt UTF-8). Il `QDataStream` usa la versione Qt 5.0 per lo schema 1, **Qt 5.2 per lo schema 2**, **Qt 5.4 per lo schema 3**; uno schema diverso da 1–3 produce l'eccezione "Unrecognized message schema".
**Numeri decimali**: nessuna chiamata a `setFloatingPointPrecision`, quindi il `float` di Qt viene scritto come **double a 8 byte** (comportamento predefinito di Qt 5): conferma la scelta di `f64`.

**Messaggi inviati da JTDX**
- `Heartbeat` (tipo 0): schema massimo (u32, vale 3) + **una sola stringa** (versione). *Non c'e' il campo "revisione" di WSJT-X: `udp-bridge` e' stato corretto per accettarne l'assenza (test scritto prima).*
- `Decode` (tipo 2): bool `new`, QTime (u32 ms), i32 SNR, double DT, u32 DF, stringa modo, stringa testo, bool bassa confidenza, bool fuori onda. Identico a quanto implementato.
- `QsoLogged` (tipo 5): QDateTime fine QSO, nominativo DX, locatore DX, u64 frequenza, modo, rapporto inviato, rapporto ricevuto, potenza, commenti, nome, QDateTime inizio, nominativo operatore, mio nominativo, mio locatore. Identico a WSJT-X.
- `WSPRDecode` (tipo 10) e `LoggedADIF` (tipo 12): presenti come in WSJT-X. `Clear`, `Status`: dello stesso tipo di WSJT-X (Status in corso di verifica).

**Messaggi accettati da JTDX** (`parse_message`)
| Tipo | Nome | Campi |
|---|---|---|
| 4 | Reply | QTime, i32 SNR, double DT, u32 DF, modo, testo, bool bassa confidenza, u8 modificatori |
| 7 | Replay | nessuno (JTDX rimanda i decode correnti) |
| 8 | HaltTx | bool `solo Tx automatico` |
| 9 | FreeText | stringa, bool `invia` |
| 13 | HighlightCallsign | nominativo, colore sfondo, colore testo, bool |
| 50 | **SetTxDeltaFreq** (solo JTDX) | i32 frequenza audio di trasmissione |
| 51 | **TriggerCQ** (solo JTDX) | stringa, bool |

**Non accettati**: `Configure` (15), `SwitchConfiguration` (14), `Location` (11) e gli altri tipi: **le impostazioni del motore non si possono cambiare via UDP in JTDX 159.**
**Controlli**: un messaggio in ingresso e' ignorato se l'**identificativo non coincide** con quello del client JTDX (va copiato dai suoi Heartbeat/Status); se lo schema in ingresso e' piu' alto di quello in uso, JTDX lo adotta.

**`Status` (tipo 1) di JTDX** (`MessageClient::status_update`, 0x56abc0): u64 frequenza, stringhe modo / DX call / rapporto / modo Tx, bool Tx abilitato / in trasmissione / in decodifica, i32 RX DF, i32 TX DF, stringhe mio nominativo / mio locatore / locatore DX, bool watchdog Tx, stringa sotto-modo, bool modo veloce e **un bool finale** (significato da identificare).
Mancano rispetto a WSJT-X: tolleranza di frequenza, periodo T/R, nome della configurazione e testo di trasmissione. Aurora deve ricavare il periodo dal modo (FT8 15 s, FT4 7,5 s) e non puo' leggere il testo Tx corrente.

**Da confermare con la cattura reale (prova manuale 1):** schema effettivamente usato da JTDX nei suoi pacchetti, significato del bool finale di `Status`, significato dei due bool del segnale `trigger_CQ(testo, bool, bool)` (il tipo 51 in ingresso legge una stringa e un bool). Il gestore e' probabilmente collegato al pulsante "Call CQ" (`on_pbCallCQ_clicked`, 0x4a4880) e a `genCQMsg` (0x4c1da0).

## Cattura reale da JTDX 2.2.159 (2026-10-24) — H1 confermata

Strumento: `udp_capture` sulla porta 2333 (il server UDP primario gia' impostato in JTDX; nessuna modifica alla configurazione). Radio accesa, FT8, durante la prova la banda e il modo sono stati cambiati (20 m, 15 m, FT4/FT8).

| Tipo | Pacchetti | Contenuto |
|---|---|---|
| 0 Heartbeat | 4 | schema massimo 3, versione `2.2.159`, nessuna revisione; circa uno ogni 15 s |
| 3 Clear | 1 | solo l'intestazione, all'avvio |
| 1 Status | 21 | uno a ogni cambio di stato (banda, modo, DF, decodifica) |
| 2 Decode | 38 (due cicli FT8 da 19) | modo `~` per FT8; DT con un decimale |

Risultati:
- **Schema 2** in tutti i pacchetti di JTDX (supporta fino al 3). Identificativo `JTDX`.
- **`udp-bridge` decodifica tutti i 64 pacchetti** senza errori; la correzione dell'Heartbeat era necessaria.
- **Status**: `dial_freq` vale 0 finche' JTDX non e' collegato alla radio (primo pacchetto all'avvio); poi la frequenza reale (14.074, 14.080, 21.074 MHz) e `dial_freq` cambia con la banda. Il **byte finale** dopo `fast_mode` vale 0 solo nel primo Status e 1 in 28 pacchetti su 29: probabilmente "monitor/ricezione attiva" (da confermare).
- **Decode**: ora in millisecondi UTC (es. 25 905 000 = 07:11:45), un lotto di 19 decode per ciclo di 15 s, `new = true`, bassa confidenza e fuori onda a falso. I messaggi con nominativo non completo compaiono come `<...>`.
- **Il filtro "prevent false UDP spots" di JTDX e' attivo** nella tua configurazione: alcuni decode con punto interrogativo non arrivano via UDP.
- Test permanenti con pacchetti reali anonimizzati: `crates/udp-bridge/tests/fixtures/` (nominativi e locatori sostituiti da valori di fantasia). Le catture complete restano in `core/captures/` (ignorata da git).

Ancora aperti: H5 (il Reply avvia davvero la sequenza), H2 (audio in parallelo), H3 (frequenza e PTT), H4 (JTDX minimizzato).

## Prova del Reply su JTDX 2.2.159 (2026-10-24) — H5 confermata, con un effetto inatteso

Procedura: Status precedente con `tx_enabled=0` e `transmitting=0`; Reply inviato con `udp_send` su un CQ decodificato 17 secondi prima (`CQ A41HA LL93`, modo "solo CQ").

Sequenza osservata negli Status di JTDX dopo il Reply:
1. `dx_call` passa da vuoto a `A41HA`, poi compare `dx_grid = LL93` e il rapporto cambia (-15 → -1): JTDX ha elaborato il decode come un doppio clic.
2. **`tx_enabled` passa a 1** e al ciclo successivo **`transmitting` passa a 1** (una sola trasmissione FT8): il Reply ha acceso Enable Tx e ha fatto partire la risposta, anche con Enable Tx spento all'inizio.
3. HaltTx inviato subito dopo: `tx_enabled` torna a 0 e `transmitting` a 0 entro un ciclo. **HaltTx funziona.**

Conseguenze:
- **Un Reply via UDP avvia da solo la trasmissione** (nella configurazione di Tiziano). Per Aurora e' la funzione che serve all'automazione dei QSO, ma va trattata come un comando che trasmette: mai inviarlo in prova senza antenna o carico fittizio.
- **Enable Tx spento non e' una protezione contro il Reply.** Le salvaguardie di Aurora (prova a vuoto, tetto di cicli, dead-man switch, STOP TX) devono stare in Aurora e non dipendere da questa impostazione.
- La trasmissione di prova e' avvenuta senza che Tiziano se l'aspettasse: errore mio nell'aver presentato il rischio come condizionato a Enable Tx. La guida alle prove e' stata corretta.
- Da verificare sul log di JTDX (ALL.TXT) il testo esatto trasmesso.

### Fine del QSO: `MainWindow::acceptQSO2` (decompilato con REA)

Ordine delle operazioni e destinazioni nella mappa `docs/udp-flussi.md`. Punto chiave: il server UDP **secondario** riceve l'ADIF come datagramma grezzo (nessuna intestazione del protocollo di WSJT-X), quindi Aurora non lo vede e non puo' sostituirlo; sul primario arrivano invece `QsoLogged` (tipo 5) e `LoggedADIF` (tipo 12), che il motore di Aurora traduce nell'evento `qso_logged`.
