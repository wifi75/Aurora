# Fase 0 — Rapporto (in compilazione)

## Esito delle ipotesi

| # | Ipotesi | Stato | Prova |
|---|---|---|---|
| H1 | JTDX 159 parla il protocollo UDP di WSJT-X | da verificare | prova manuale 1 (`fase0-prove-manuali.md`) |
| H2 | Cattura audio in parallelo a JTDX | da verificare | prova 3; unico ingresso visto: `Microfono (2- USB Audio Device)` |
| H3 | Lettura/scrittura frequenza e PTT | da verificare | prova 4; `rigctld-jtdx` supporta `-m -r -s -t` e le porte TCP (aiuto verificato) |
| H4 | JTDX stabile minimizzato | da verificare | prova 5 |
| H5 | `Reply` UDP avvia un QSO completo | da verificare | prova 2 |

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
