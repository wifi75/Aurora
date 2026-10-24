# Fase 0 — Prove manuali (a cura di Tiziano)

Eseguire in ordine. Scrivere l'esito di ogni prova in `docs/fase0-rapporto.md`. Tutti i comandi partono da `Aurora\core`
dopo `cargo build -p probes`. **Prima di ogni prova che manda comandi a JTDX: disattivare Enable Tx e abbassare la potenza
della radio (o usare un carico fittizio).**

## Prova 1 — Cattura UDP da JTDX (ipotesi H1)

1. In JTDX: Impostazioni → Reporting → UDP Server: indirizzo `127.0.0.1`, porta `2237`. Per la prova 2 attiva anche **"Accept UDP requests"** e, dal menu, **"Accept UDP Reply messages"** (il reverse di `jtdx.exe` mostra che senza queste opzioni il Reply viene ignorato).
2. In un terminale: `.\target\debug\udp_capture.exe 2237 crates\udp-bridge\tests\fixtures`
3. Lasciare JTDX in ascolto su una banda attiva (es. 20 m, 14.074 MHz) per almeno 5 cicli (75 secondi).
4. Atteso: righe con tipo `0` (Heartbeat), `1` (Status), `2` (Decode). Fermare con Ctrl+C.
5. Annotare: i tipi comparsi, la colonna "da" (indirizzo di JTDX, servirà alla prova 2) e se ci sono tipi diversi.
6. Poi: togliere `#[ignore]` da `every_real_capture_decodes_and_is_plausible` in `crates/udp-bridge/tests/captured.rs`
   e dirmi di eseguire `cargo test -p udp-bridge`: se fallisce, il formato di JTDX differisce da quello ipotizzato e lo correggo.

## Prova 2 — Comandi verso JTDX (ipotesi H5)

Con Enable Tx **spento**:
1. Scegliere un file `Status` (`*-1.bin`) e un file `Decode` (`*-2.bin`) tra quelli catturati.
2. `.\target\debug\udp_send.exe <indirizzo-da-prova-1> <status.bin> halt` — JTDX non deve dare errori.
3. `.\target\debug\udp_send.exe <indirizzo> <status.bin> reply <decode.bin>` — atteso: JTDX compila DX Call e prepara la
   sequenza di risposta alla stazione, come con un doppio clic. Annotare cosa succede e se la trasmissione parte solo con Enable Tx.

Note dal reverse di JTDX (vedi `fase0-rapporto.md`): invia il `reply` **mentre il decode e' ancora visibile** nella finestra di JTDX (il decode viene ritrovato dal testo); scegli nel menu "Accept UDP Reply messages" il modo "qualsiasi"; attiva il log di debug di JTDX e controlla ALL.TXT: devono comparire "UDP Reply request received" e "processed". Se compare "decode not found", il decode non e' piu' in finestra o il formato dell'ora/DT non coincide: annota la riga esatta.

## Prova 3 — Audio in parallelo (ipotesi H2)

L'unico ingresso audio visto da Aurora è `Microfono (2- USB Audio Device)`.
1. Con JTDX aperto e in ricezione su quell'ingresso: `.\target\debug\audio_tone.exe "USB Audio"`
2. Atteso: ogni secondo una riga con "picco … Hz" e livello, **mentre JTDX continua a decodificare**.
3. Se fallisce, copiare il messaggio d'errore esatto.

## Prova 4 — Frequenza e PTT (ipotesi H3)

1. Lo `Status` catturato contiene la frequenza di dial: confrontarla con la radio e annotare se si aggiorna al cambio banda.
2. Prova opzionale del percorso di scrittura: chiudere JTDX, avviare `rigctld-jtdx.exe -m <modello> -r <COM> -s <baud> -t 4532`
   (modello e porta della tua radio: `rigctld-jtdx.exe -l` elenca i modelli), poi impostare in JTDX il controllo radio
   "Hamlib NET rigctl" su `127.0.0.1:4532`. Verificare che JTDX controlli la radio e che da un secondo terminale
   si legga la frequenza con `echo f | ncat 127.0.0.1 4532`.

## Prova 5 — JTDX in secondo piano (ipotesi H4)

Avviare JTDX minimizzato e lasciare `udp_capture` attivo un'ora: i datagrammi continuano ad arrivare? Annotare interruzioni.
