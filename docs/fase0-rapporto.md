# Fase 0 — Rapporto (in compilazione)

## Esito delle ipotesi

| # | Ipotesi | Stato | Prova |
|---|---|---|---|
| H1 | JTDX 159 parla il protocollo UDP di WSJT-X | **supportata dal reverse** (formato esatto ancora da confermare con la cattura) | prova manuale 1 (`fase0-prove-manuali.md`) |
| H2 | Cattura audio in parallelo a JTDX | da verificare | prova 3; unico ingresso visto: `Microfono (2- USB Audio Device)` |
| H3 | Lettura/scrittura frequenza e PTT | da verificare | prova 4; `rigctld-jtdx` supporta `-m -r -s -t` e le porte TCP (aiuto verificato) |
| H4 | JTDX stabile minimizzato | da verificare | prova 5 |
| H5 | `Reply` UDP avvia un QSO completo | **supportata dal reverse** (serve l'opzione "Accept UDP requests"; avvio del QSO da confermare con la prova 2) | prova 2 |

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
