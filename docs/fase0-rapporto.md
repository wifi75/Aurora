# Fase 0 — Rapporto (in compilazione)

## Esito delle ipotesi

| # | Ipotesi | Stato | Prova |
|---|---|---|---|
| H1 | JTDX 159 parla il protocollo UDP di WSJT-X | da verificare | serve la cattura reale (Task 2, Step 5) |
| H2 | Cattura audio in parallelo a JTDX | da verificare | Task 5 |
| H3 | Lettura/scrittura frequenza e PTT | da verificare | Task 6 |
| H4 | JTDX stabile minimizzato | da verificare | Task 6 |
| H5 | `Reply` UDP avvia un QSO completo | da verificare | Task 4, Step 7 |

## Decisioni prese durante l'esecuzione

- **Task 1:** con `members = ["crates/*"]` e nessun crate, `cargo metadata` fallisce su Windows (il glob non trova corrispondenze). La verifica si fa dopo il Task 2, quando esiste il primo crate.
- **Task 3:** `decode` restituisce `Packet { schema, message }` invece di `Message` e non esiste il tipo `Header`: lo schema serve ai comandi del Task 4 e l'identificativo sta già in ogni messaggio.
- **Task 3:** niente `Error::Unsupported`: lo schema non cambia il layout dei messaggi modellati, e i tipi sconosciuti producono `Message::Unknown` con i byte originali.
- **Task 3:** i campi finali di `Status` (da `special_op_mode` in poi) sono opzionali, perché versioni o schemi più vecchi di JTDX possono non averli; il test di troncamento richiede l'errore solo prima della fine della parte obbligatoria.
- **Task 3:** il test sulle catture reali è `#[ignore]` con motivo esplicito finché `tests/fixtures/` è vuota; va riabilitato dopo la cattura.

## Differenze tra pacchetti reali e il formato ipotizzato

(da compilare dopo la cattura)
