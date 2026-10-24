# Aurora — Piano 1: repository e Fase 0 (spike) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Creare il repository Aurora e verificare con prove reali le ipotesi H1–H5 su JTDX 159, producendo il crate `udp-bridge` (riusato dalle fasi successive) e un rapporto di fattibilità che decide il Piano 2.

**Architecture:** Workspace Rust in `core/`. Il crate libreria `udp-bridge` implementa codifica e decodifica del protocollo UDP di WSJT-X (Qt `QDataStream`, big-endian) con test guidati da pacchetti. Il crate `probes` contiene tre binari usa-e-getta (cattura UDP, invio comandi, tono audio) per le prove manuali con JTDX. Nessuna dipendenza esterna nel crate `udp-bridge`.

**Tech Stack:** Rust edition 2024 (rustc 1.98.0), `cpal 0.18.2`, `rustfft 6.4.1` (solo nei probe). Test con `cargo test`.

**Spec:** `docs/2026-10-24-aurora-design.md` (§2 ipotesi, §4 componenti, §12 Fase 0)

## Global Constraints

- Rust edition 2024, rustc 1.98.0 (installato); versioni dei crate verificate sul registro il 2026-10-24: `cpal 0.18.2`, `rustfft 6.4.1`.
- Il repository non contiene la cartella locale degli strumenti di assistenza né riferimenti a tali strumenti, **nemmeno nei trailer dei commit**. Il file di istruzioni si chiama `AGENTS.md`. La regola di esclusione di quella cartella sta in `.git/info/exclude`, non nel `.gitignore` pubblicato.
- Nessun segreto nel repository; `.env.example` senza valori reali.
- Nessun `.unwrap()` / `.expect()` fuori dai test; gli errori sono `Result` con un tipo di errore proprio.
- Logging con `tracing` quando serve nel core; nei probe è ammesso `println!` (strumenti usa-e-getta, dichiarati tali).
- Piè di pagina dell'interfaccia: "Aurora vX.Y.Z — Progettazione e sviluppo: Tiziano Cassone" (non riguarda questo piano, ma `APP_AUTHOR` va già definito in un solo punto: `core/crates/udp-bridge` non lo usa, lo definisce il futuro crate `remote-api`).
- Le prove manuali con JTDX, la radio e il telefono le esegue Tiziano; gli agenti eseguono solo i test automatici e preparano i comandi esatti.
- Il formato dei pacchetti qui sotto viene da `NetworkMessage.hpp` di WSJT-X: **è un'ipotesi finché non è confermato dalle catture reali di JTDX 159** (Task 3).

## Review Focus

1. Pacchetto troncato o più corto del dichiarato → errore `Truncated`, mai panico.
2. Stringa Qt nulla (`0xFFFFFFFF`) e stringa vuota → rappresentate in modo distinto (`None` / `Some("")`).
3. Lunghezza stringa enorme (es. 4 GB) in un datagramma da 40 byte → errore, nessuna allocazione gigante.
4. Tipo di messaggio sconosciuto o schema diverso da quelli attesi → `Unknown`/`Unsupported`, non un crash; JTDX può usare un tipo non previsto.
5. UTF-8 non valido in un messaggio di decode → errore gestito oppure sostituzione, mai panico; un datagramma da un mittente non JTDX sulla stessa porta viene ignorato dalla cattura.

## File Structure

```
Aurora/
  .gitignore  AGENTS.md  README.md  CHANGELOG.md  TODO.md  memory.md  .env.example
  core/
    Cargo.toml                      # workspace
    crates/udp-bridge/
      Cargo.toml
      src/lib.rs                    # re-export e Error
      src/wire.rs                   # Reader/Writer big-endian (u8,u32,u64,i32,f64,bool,utf8)
      src/message.rs                # enum Message + decode(&[u8]) / encode(&Message)
      tests/fixtures/               # pacchetti .bin catturati da JTDX (Task 3)
      tests/roundtrip.rs            # test di codifica/decodifica e di robustezza
      tests/captured.rs             # test sui pacchetti reali
    crates/probes/
      Cargo.toml
      src/bin/udp_capture.rs        # ascolta e salva i datagrammi
      src/bin/udp_send.rs           # invia Reply / HaltTx / FreeText
      src/bin/audio_tone.rs         # cattura audio e stampa la frequenza di picco
  docs/
    2026-10-24-aurora-design.md
    2026-10-24-aurora-fase0-spike-plan.md
    fase0-rapporto.md               # esito H1–H5 e decisione (Task 7)
```

---

### Task 1: Repository e documentazione di base

**Files:**
- Create: `Aurora/.gitignore`, `Aurora/AGENTS.md`, `Aurora/CHANGELOG.md`, `Aurora/TODO.md`, `Aurora/memory.md`, `Aurora/.env.example`, `Aurora/core/Cargo.toml`
- Modify: `Aurora/README.md` (stato e comandi)

**Interfaces:**
- Produces: workspace `core/Cargo.toml` con `members = ["crates/*"]`, `resolver = "3"`, `[workspace.package] edition = "2024"`, `version = "0.0.1"`.

- [ ] **Step 1: Inizializzare git.** In `Aurora/`: `git init -b main`; aggiungere la cartella locale degli strumenti di assistenza a `.git/info/exclude` (non al `.gitignore`).
- [ ] **Step 2: Scrivere i file di base.** `.gitignore` con `target/`, `*.log`, `.env`, `captures/`; `AGENTS.md` con regole del progetto (stack, comandi `cargo test`, nessun `unwrap` fuori dai test, mai riferimenti a strumenti di assistenza); `README.md` aggiornato; `CHANGELOG.md` con voce `0.0.1`; `TODO.md` con le fasi della specifica §12; `memory.md` con decisioni prese (opzione A, JTDX come motore, tutti e tre i trasporti); `.env.example` con le sole chiavi vuote `AURORA_UDP_PORT=2237`.
- [ ] **Step 3: Workspace vuoto compila.** Run: `cd core && cargo metadata --no-deps` — Expected: esce con 0 (nessun membro ancora, ok).
- [ ] **Step 4: Verificare l'assenza di riferimenti agli strumenti di assistenza** con una ricerca testuale dei loro nomi sui file tracciati — Expected: nessun risultato.
- [ ] **Step 5: Commit** `chore: struttura iniziale del repository` (nessun trailer).

---

### Task 2: Strumento di cattura UDP (H1, lato ricezione)

**Files:**
- Create: `core/crates/probes/Cargo.toml`, `core/crates/probes/src/bin/udp_capture.rs`

**Interfaces:**
- Produces: binario `udp_capture` — `udp_capture <porta> <cartella>` ascolta su `0.0.0.0:<porta>`, salva ogni datagramma come `<cartella>/NNNN-<tipo>.bin` (tipo = i 4 byte alle posizioni 8..12 letti come u32 big-endian, `?` se corto) e stampa una riga per datagramma: indice, mittente, lunghezza, tipo.

- [ ] **Step 1: Scrivere il probe** con `std::net::UdpSocket`; porta in uso → messaggio chiaro e uscita con codice 1; datagramma < 12 byte → salvato ma tipo `?`.
- [ ] **Step 2: Compila.** Run: `cd core && cargo build -p probes --bin udp_capture` — Expected: successo senza avvisi.
- [ ] **Step 3: Prova locale a vuoto.** Run in due terminali: `cargo run -p probes --bin udp_capture -- 2237 captures/prova` e `powershell -c "$u=New-Object Net.Sockets.UdpClient; $b=[byte[]](1..20); $u.Send($b,20,'127.0.0.1',2237)"` — Expected: una riga con lunghezza 20 e un file `0001-*.bin` di 20 byte.
- [ ] **Step 4: Commit** `feat(probes): cattura datagrammi UDP`.
- [ ] **Step 5: PROVA MANUALE (Tiziano), esito in `docs/fase0-rapporto.md`:** in JTDX, Impostazioni → Reporting, impostare UDP Server `127.0.0.1` porta `2237`, lasciare lavorare JTDX su una banda attiva per alcuni cicli, quindi eseguire `udp_capture 2237 core/crates/udp-bridge/tests/fixtures`. Risultato atteso: file di tipo 0 (Heartbeat), 1 (Status), 2 (Decode). **Questo conferma o smentisce H1 in ricezione.**

---

### Task 3: Crate `udp-bridge` — decodifica dei messaggi (H1)

**Files:**
- Create: `core/crates/udp-bridge/Cargo.toml`, `src/lib.rs`, `src/wire.rs`, `src/message.rs`, `tests/roundtrip.rs`, `tests/captured.rs`
- Test: `core/crates/udp-bridge/tests/roundtrip.rs`, `tests/captured.rs`

**Interfaces:**
- Produces (`pub`):
  - `enum Error { Truncated, BadMagic(u32), BadUtf8, TooLong(u32), Unsupported(u32) }` con `Display` e `std::error::Error`.
  - `struct Header { schema: u32, kind: u32, id: Option<String> }`.
  - `enum Message { Heartbeat{id, max_schema: u32, version: Option<String>, revision: Option<String>}, Status(Status), Decode(Decode), Clear{id, window: Option<u8>}, QsoLogged{id, call: Option<String>, ...}, Close{id}, Unknown{kind: u32, id: Option<String>, raw: Vec<u8>} }` — i campi non elencati si ricavano da `NetworkMessage.hpp` e dalle catture.
  - `struct Decode { id: Option<String>, is_new: bool, time_ms: u32, snr: i32, delta_time: f64, delta_freq: u32, mode: Option<String>, message: Option<String>, low_confidence: bool, off_air: bool }`.
  - `struct Status { id: Option<String>, dial_freq: u64, mode: Option<String>, dx_call: Option<String>, report: Option<String>, tx_mode: Option<String>, tx_enabled: bool, transmitting: bool, decoding: bool, rx_df: u32, tx_df: u32, de_call: Option<String>, de_grid: Option<String>, dx_grid: Option<String>, tx_watchdog: bool, submode: Option<String>, fast_mode: bool, ... }`.
  - `fn decode(datagram: &[u8]) -> Result<Message, Error>`.
  - Costante `MAGIC: u32 = 0xadbccbda`.
- Costanti dei tipi: `HEARTBEAT=0, STATUS=1, DECODE=2, CLEAR=3, REPLY=4, QSO_LOGGED=5, CLOSE=6, REPLAY=7, HALT_TX=8, FREE_TEXT=9, WSPR_DECODE=10, LOCATION=11, LOGGED_ADIF=12`.

- [ ] **Step 1: Test di `wire`.** In `src/wire.rs` (moduli di test interni): `reader_reads_big_endian_u32`, `reader_utf8_null_is_none` (`FF FF FF FF` → `Ok(None)`), `reader_utf8_empty_is_some_empty` (`00 00 00 00` → `Some("")`), `reader_truncated_is_error`, `reader_huge_length_is_too_long_without_allocating` (lunghezza `FFFFFFF0` in 20 byte → `Err(TooLong)`).
- [ ] **Step 2: Run, verificare il fallimento.** `cargo test -p udp-bridge wire` — Expected: FAIL (non compila / non definito).
- [ ] **Step 3: Implementare `Reader`** (`u8, u32, u64, i32, f64, bool, utf8`, tutte restituiscono `Result<_, Error>` e non fanno mai indicizzazione non controllata) e `Writer` speculare, in `src/wire.rs`.
- [ ] **Step 4: Run, verificare il successo.** `cargo test -p udp-bridge wire` — Expected: PASS.
- [ ] **Step 5: Test di `decode` su pacchetti sintetici** in `tests/roundtrip.rs`, costruiti con `Writer` secondo `NetworkMessage.hpp`: `decode_heartbeat`, `decode_status_all_fields`, `decode_decode_message` (es. `"CQ DX IU3CYV JN55"`, snr `-12`, delta_freq `1500`, delta_time `0.3`), `bad_magic_is_error`, `unknown_kind_is_unknown_not_error`, `truncated_status_is_error_at_every_cut` (tutte le lunghezze da 0 a n-1 devono restituire `Err`, mai panico), `invalid_utf8_in_message_is_bad_utf8`.
- [ ] **Step 6: Run, verificare il fallimento.** `cargo test -p udp-bridge --test roundtrip` — Expected: FAIL.
- [ ] **Step 7: Implementare `decode` e i tipi** in `src/message.rs`. Il tipo `Unknown` conserva i byte grezzi così nulla si perde nei tipi non ancora modellati.
- [ ] **Step 8: Run, verificare il successo.** `cargo test -p udp-bridge --test roundtrip` — Expected: PASS.
- [ ] **Step 9: Test sulle catture reali** in `tests/captured.rs`: per ogni `.bin` in `tests/fixtures/` (prodotti dal Task 2) `decode` deve restituire `Ok` e, per i tipi 1 e 2, i campi devono essere plausibili (frequenza tra 1 MHz e 450 MHz; testo del decode non vuoto). **Se un pacchetto reale non coincide con il layout ipotizzato, correggere `message.rs` e i test sintetici in base ai byte reali** (annotare la differenza in `docs/fase0-rapporto.md`). Se `fixtures/` è vuota il test è `#[ignore]` con messaggio esplicito, non un successo silenzioso.
- [ ] **Step 10: Run completo.** `cargo test -p udp-bridge` — Expected: tutti PASS (i test su catture eseguiti, non ignorati, se le catture esistono).
- [ ] **Step 11: Commit** `feat(udp-bridge): decodifica dei messaggi UDP di JTDX`.

---

### Task 4: Codifica dei comandi e invio (H5)

**Files:**
- Modify: `core/crates/udp-bridge/src/message.rs`, `src/lib.rs`
- Create: `core/crates/probes/src/bin/udp_send.rs`
- Test: `core/crates/udp-bridge/tests/roundtrip.rs`

**Interfaces:**
- Consumes: `Writer`, `MAGIC`, costanti dei tipi (Task 3).
- Produces: `fn encode_halt_tx(id: &str, schema: u32, auto_tx_only: bool) -> Vec<u8>`; `fn encode_free_text(id: &str, schema: u32, text: &str, send: bool) -> Vec<u8>`; `fn encode_reply(id: &str, schema: u32, decode: &Decode, modifiers: u8) -> Vec<u8>` (rimanda al decode originale: time, snr, delta_time, delta_freq, mode, message, low_confidence).

- [ ] **Step 1: Test.** `encode_halt_tx_layout` (magic, schema, tipo 8, id, 1 byte bool), `encode_reply_echoes_decode_fields` (decodifica a ritroso con `Reader` e confronta i campi con il `Decode` d'ingresso), `encode_free_text_utf8`.
- [ ] **Step 2: Run, verificare il fallimento.** `cargo test -p udp-bridge --test roundtrip encode` — Expected: FAIL.
- [ ] **Step 3: Implementare le tre funzioni** con firma sopra.
- [ ] **Step 4: Run, verificare il successo.** Stesso comando — Expected: PASS.
- [ ] **Step 5: Probe `udp_send`.** `udp_send <host:porta> halt|free "<testo>"|reply <file-decode.bin>`: invia il pacchetto a JTDX usando `id` e `schema` letti dall'ultimo `Status` catturato. Compila: `cargo build -p probes --bin udp_send`.
- [ ] **Step 6: Commit** `feat(udp-bridge): codifica di Reply, HaltTx e FreeText`.
- [ ] **Step 7: PROVA MANUALE (Tiziano):** con JTDX in ascolto su una banda attiva e **trasmissione automatica disabilitata e potenza a zero / carico fittizio**: (a) inviare `halt` e verificare che JTDX non dia errori; (b) inviare `reply` su un decode catturato e verificare che JTDX prepari la risposta alla stazione (campo DX Call compilato, sequenza TX avviata solo se Enable Tx attivo). **Esito = H5** in `fase0-rapporto.md`.

---

### Task 5: Cattura audio in parallelo a JTDX (H2)

**Files:**
- Create: `core/crates/probes/src/bin/audio_tone.rs`; modify `core/crates/probes/Cargo.toml` (dipendenze `cpal = "0.18.2"`, `rustfft = "6.4.1"`)

**Interfaces:**
- Produces: binario `audio_tone` — `audio_tone [nome-dispositivo-parziale]` apre il dispositivo di ingresso (predefinito se omesso), calcola una FFT su finestre da 8192 campioni e stampa ogni secondo: dispositivo, frequenza di campionamento, frequenza di picco in Hz, livello in dB. Con dispositivo non trovato o ingresso non disponibile stampa l'elenco dei dispositivi e termina con codice 1.
- Test (automatico, in `audio_tone.rs`): funzione pura `fn peak_hz(samples: &[f32], sample_rate: u32) -> f32` con test `peak_of_1500hz_sine_is_1500hz` (±`sample_rate/8192` Hz) e `peak_of_silence_is_none_or_zero`.

- [ ] **Step 1: Test di `peak_hz`** con un seno sintetico a 1500 Hz e 12 kHz di campionamento.
- [ ] **Step 2: Run, verificare il fallimento.** `cargo test -p probes peak_hz` — Expected: FAIL.
- [ ] **Step 3: Implementare `peak_hz`** con `rustfft` (finestra di Hann prima della FFT), poi il ciclo di cattura `cpal`.
- [ ] **Step 4: Run, verificare il successo.** `cargo test -p probes` — Expected: PASS.
- [ ] **Step 5: Commit** `feat(probes): picco di frequenza dall'ingresso audio`.
- [ ] **Step 6: PROVA MANUALE (Tiziano):** JTDX aperto e in ricezione sullo stesso ingresso audio; avviare `audio_tone`; riprodurre un tono da 1500 Hz (o ascoltare FT8 reale). Risultato atteso: `audio_tone` stampa il picco **mentre JTDX continua a decodificare**, senza errori su nessuno dei due. **Esito = H2.** Se fallisce, annotare il messaggio d'errore esatto e provare l'ingresso in modalità condivisa di un altro dispositivo.

---

### Task 6: Controllo della radio (H3) e finestra di JTDX (H4)

**Files:**
- Create/Modify: sezione H3/H4 di `docs/fase0-rapporto.md` (nessun codice nuovo se non serve)

**Interfaces:** nessuna produzione di codice; produce la decisione su come Aurora legge/imposta frequenza e PTT.

- [ ] **Step 1: Leggere la frequenza dal `Status` UDP** con `udp_capture` (campo `dial_freq` già decodificato nel Task 3) e confrontarla con la radio. Annotare: la frequenza è corretta? Si aggiorna al cambio banda?
- [ ] **Step 2: Valutare il percorso di scrittura.** Verificare nella documentazione/sorgenti di JTDX se il messaggio `Configure` (tipo 15) permette di cambiare banda/frequenza; se no, ipotizzare di **far possedere `rigctld-jtdx.exe` ad Aurora** e di configurare JTDX come rig "Hamlib NET rigctl" verso `127.0.0.1:4532`, così entrambi condividono la radio.
- [ ] **Step 3: PROVA MANUALE (Tiziano) del percorso scelto:** avviare `rigctld-jtdx.exe` (parametri esatti nel rapporto) con la porta COM della radio, configurare JTDX su NET rigctl, poi da un terminale `echo f | nc 127.0.0.1 4532` (o equivalente PowerShell) per leggere la frequenza. **Esito = H3.**
- [ ] **Step 4: Prova H4:** avviare JTDX minimizzato (oppure con un collegamento impostato su "minimizzata") e far girare un'ora con `udp_capture`: i datagrammi continuano ad arrivare? **Esito = H4.**
- [ ] **Step 5: Inventario delle funzioni di JTDX (parità funzionale, spec §1.1).** Creare `docs/fase0-parita-funzioni.md`: tabella con una riga per funzione (menu, schede di Impostazioni, filtri, tasti rapidi) e le colonne *funzione*, *dove sta in JTDX*, *via UDP / solo JTDX / da riscrivere*, *priorità*. Fonti: finestre di `jtdx.exe` esplorate da Tiziano, manuale JTDX, sorgenti pubblici. Terminare con la lista delle funzioni nuove proposte, da far scegliere a Tiziano.
- [ ] **Step 6: Commit** del rapporto parziale `docs: esiti H3 e H4 e inventario funzioni`.

---

### Task 7: Rapporto di fattibilità e decisione

**Files:**
- Create: `docs/fase0-rapporto.md`; modify: `TODO.md`, `CHANGELOG.md`, `memory.md`

**Interfaces:** produce la decisione sul Piano 2 (core in sola lettura).

- [ ] **Step 1: Compilare la tabella H1–H5** (verificata / smentita / parziale, con la prova) e le differenze trovate tra i pacchetti reali e `NetworkMessage.hpp`.
- [ ] **Step 2: Decidere sull'uso di REA + Ghidra.** Se H1 o H5 sono smentite o ambigue: installare Ghidra, eseguire `npx rea-agents setup` (Node 24.18.0 presente) e analizzare `jtdx.exe` per il gestore dei messaggi UDP; annotare i risultati. Altrimenti scrivere "non necessario" con la motivazione.
- [ ] **Step 3: Scrivere la decisione:** *procedere* con l'opzione A come da specifica / *adattare* (indicando cosa cambia) / *fermarsi*. Aggiornare le domande aperte della specifica §14 (licenza, dominio).
- [ ] **Step 4: Aggiornare `TODO.md`, `CHANGELOG.md` (0.1.0 "Fase 0 conclusa"), `memory.md`.**
- [ ] **Step 5: Verifica finale.** `cargo test --workspace` — Expected: tutti PASS; `git status` pulito dopo il commit.
- [ ] **Step 6: Commit** `docs: rapporto Fase 0`; tag `v0.1.0`.

---

## Self-review

- **Copertura della specifica:** H1 → Task 2–3; H5 → Task 4; H2 → Task 5; H3, H4 → Task 6; REA/Ghidra condizionale → Task 7; §10 (struttura, file di progetto) → Task 1; §11 (test su pacchetti reali, segnali sintetici) → Task 3 e 5. Le fasi 1–6 sono volutamente rinviate a piani successivi, scritti con i risultati del rapporto.
- **Coerenza dei tipi:** `Decode` è definito nel Task 3 e riusato da `encode_reply` nel Task 4; `Writer`, `MAGIC` e le costanti dei tipi nascono nel Task 3 e vengono consumati nel Task 4.
- **Review Focus** coperti da: test di troncamento a ogni lunghezza, stringa nulla/vuota, lunghezza enorme, tipo sconosciuto, UTF-8 non valido (tutti nel Task 3), porta già in uso e dispositivo audio assente (Task 2 e 5).
- **Rischio dichiarato:** il layout dei pacchetti è un'ipotesi finché non è confrontato con le catture (Task 3, Step 9); è per questo che i test sintetici e quelli sulle catture sono separati.
