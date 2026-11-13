# Prestazioni e stabilita' del motore (CPU, memoria, eventi)

Verifica del 2026-11-13 sulla 0.11.243 (`5c73104d`). Regola seguita: nessuna modifica senza un numero prima e dopo, e nessuna modifica che
cambi i risultati: ogni prova di misura stampa un'**impronta** (FNV-1a) di messaggi, campioni, righe o immagini, che deve restare identica.

## Come si rifanno le misure
Tutte in `--release`, prove `#[ignore]` con `--nocapture` (PC di sviluppo, 16 processori; i numeri sono ripetibili entro il 3-5%):

| Cosa | Comando (`cd core`) |
|---|---|
| FT8, un periodo (1 e 16 thread) | `cargo test -p ft8 --release --test misure -- --ignored --nocapture prestazioni_periodo` |
| FT4, un periodo | `cargo test -p ft4 --release --test prestazioni -- --ignored --nocapture` |
| Ricampionamento del motore (scheda -> 12 kHz) | `cargo test -p engine-native --release --lib resample::tests::prestazioni -- --ignored --nocapture` |
| Cascata e livello | `cargo test -p audio-spectrum --release --test prestazioni -- --ignored --nocapture` |
| PSK31, PSK63, RTTY, CW (un ricevitore) | `cargo test -p mode-digi --release --test prestazioni -- --ignored --nocapture` |
| Tutta la banda CW e PSK (10 e 30 segnali) | `cargo test -p mode-digi --release --test cw_confronto --test psk_confronto -- --ignored --nocapture cpu_tutta_la_banda tutti_i_segnali` |
| SSTV e ricampionamento di SSTV/modi testuali | `cargo test -p mode-sstv --release --test prestazioni -- --ignored --nocapture` |
| Peso degli eventi JSON verso l'interfaccia | `cargo test -p aurora-core --release --test prestazioni -- --ignored --nocapture` |

## Tabella prima/dopo
CPU in millisecondi di calcolo per secondo di audio (10 ms/s = 1% di un core). "=" vuol dire impronta identica prima e dopo.

| Misura | Prima | Dopo | Impronta |
|---|---|---|---|
| FT8 profondita' 3, 12 periodi, 1 thread (media di due giri alternati) | 782 ms/periodo | 759 ms/periodo (-3%) | = `bdd035dff462f222`, 116 messaggi |
| FT8 profondita' 3, 16 thread (due giri alternati) | 342 ms/periodo | 321-344 ms/periodo (entro il rumore) | = |
| FT4, 12 periodi (due giri alternati) | 71 ms/periodo | 67,5 ms/periodo (-5%) | = `8b9158e2c8e2c061`, 96 messaggi |
| Ricampionamento del motore 48 kHz -> 12 kHz (sempre acceso) | 21,0 ms/s (2,1%) | 1,07 ms/s (0,11%) | = |
| Ricampionamento del motore 96 kHz | 42,6 ms/s (4,3%) | 2,2 ms/s (0,22%) | = |
| Ricampionamento del motore 44,1 kHz | 19,6 ms/s | 19,1 ms/s (invariato: rapporto non intero) | = |
| Ricampionamento SSTV/modi testuali 48 kHz (uno per servizio acceso) | 2,25 ms/s | 1,50 ms/s (-33%) | = |
| Ricampionamento SSTV/modi testuali 44,1 kHz | 2,10 ms/s | 1,41 ms/s | = |
| Cascata 48 kHz (FFT 65536, 15 righe/s) | 4,1 ms/s | invariata | = |
| Cascata 44,1 kHz (FFT 60211, non potenza di 2) | 8,8 ms/s | invariata | = |
| Cascata 96 kHz (FFT 131072) | 7,7-10,9 ms/s | invariata | = |
| Misuratore di livello 48 kHz | 0,06 ms/s | invariato | |
| SSTV Robot36 / Martin1 (12 kHz) | 5,1 ms/s | invariato (4,5-4,7, rumore della misura) | = |
| PSK31 / PSK63, un ricevitore | 0,42 / 0,56 ms/s | invariato | |
| RTTY, un ricevitore (tre catene) | 2,4 ms/s | invariato | |
| CW, un ricevitore (bayesiano + AGC) | 21,2 ms/s (2,1%) | 11,6 ms/s (1,2%), vedi «CW di tutta la banda più leggero» | = `609fc49e09fc0b1c`, 398 caratteri |
| Tutta la banda CW, 10 / 30 stazioni | 8,6% / 25,1% di un core | 5,4% / 16,9% (-37%) | = (stessi canali, nominativi e caratteri) |
| Tutta la banda PSK31, 10 / 30 stazioni | 0,4% / 1,1% di un core | invariato | |

Con il motore proprio acceso e una scheda a 48 kHz il lavoro fisso (ricampionamento + cascata) scende da circa 25 a circa 5 ms per secondo di
audio; con 96 kHz da circa 50-53 a circa 10.

### Eventi verso l'interfaccia (per client WebSocket)
Un `serde_json::to_string` per evento e per client (`remote-api/src/ws.rs:30`):

| Evento | Byte | Serializzazione | Frequenza tipica | Traffico |
|---|---|---|---|---|
| Riga di spettro (binaria) | 6844 | nessuna (copia) | 15/s | 103 kB/s |
| `Digi` `band`, 30 segnali | 6214 | 8,7 us | 2/s | 12,4 kB/s |
| `Digi` `band`, 10 segnali | 2094 | 3,5 us | 2/s | 4,2 kB/s |
| `Sstv` `rows` (Robot36, 2 righe in base64) | 2647 | 1,1 us | ~3/s | 8,7 kB/s |
| `Decode` (FT8) | 220 | 0,6 us | ~3/s su banda piena | 0,6 kB/s |
| `Status` | 262 | 0,8 us | ~1/s | 0,3 kB/s |
| `AudioLevel` | 74 | 0,3 us | 4/s | 0,3 kB/s |

La CPU della serializzazione e' trascurabile (meno di 0,03 ms al secondo per client); il peso vero e' la cascata (~100 kB/s per client, gia'
binaria), che conta solo per il telefono su rete mobile.

## Cosa e' stato corretto

### Prestazioni (risultati identici al bit)
1. **Ricampionamento del motore** (`core/crates/engine-native/src/resample.rs`, ciclo in `push`): per ogni campione in uscita (12000/s) e per
   ogni coefficiente (128 a 48 kHz, 256 a 96 kHz) si calcolavano un coseno, un seno e una divisione in f64: 1,5-3 milioni di funzioni
   trigonometriche al secondo, sempre, anche senza decodificare. Con un rapporto intero il centro del filtro cade sempre su un campione e i
   coefficienti sono sempre gli stessi: ora vengono da una tabella calcolata all'avvio con le stesse operazioni (stessi bit; prova
   `the_table_gives_the_same_bits_as_the_formula` contro la formula di prima). 44,1 kHz resta com'era.
2. **Ricampionamento di SSTV e modi testuali** (`core/crates/mode-sstv/src/resample.rs`, `process`): due resti di divisione (`% TAPS`) per
   coefficiente, 63 coefficienti per campione d'ingresso. Lo storico ora e' scritto due volte di fila e la somma scorre una fetta contigua,
   nello stesso ordine (stessi bit).
3. **FFT dei decoder FT8 e FT4** (`core/crates/ft8/src/sync.rs:125`, `core/crates/ft4/src/sync.rs:95`): `fft_plan` creava un pianificatore
   nuovo a ogni chiamata (piu' volte per passata e, in FT4, per ogni candidato in `bit_metrics`), ricalcolando piano e tabelle. Ora un
   pianificatore per thread ricorda i piani.
4. **FT4, tabelle per periodo** (`core/crates/ft4/src/decode.rs:154-155`): riferimenti di Costas (`SyncTables`) e filtro della sottrazione
   (`Subtractor`, due FFT da 73728 punti e il loro nucleo) si ricalcolavano a ogni periodo; sono costanti e ora si calcolano una volta.

### Stabilita' (ogni correzione ha una prova che falliva prima e passa dopo)
5. **`digi.json` con valori impossibili fermava per sempre la ricezione dei modi testuali** (`core/crates/mode-digi/src/service.rs`,
   `DigiService::open`): il file si leggeva senza `validate()`; con `rtty_baud` a 0 (file scritto a mano o rovinato) il thread `aurora-digi`
   andava in panic all'avvio ("capacity overflow" in `rtty.rs`, decimazione `usize::MAX`) e PSK, RTTY e CW non ricevevano piu' niente, senza
   avviso. Ora i valori fuori dai limiti tornano quelli di base uno per uno (`DigiSettings::repaired`), il resto resta. Prova
   `un_digi_json_con_valori_impossibili_non_ferma_la_ricezione`.
6. **Un campione NaN o infinito bloccava per sempre i ricevitori dei modi testuali** (`DigiService::feed`): un solo valore non finito (float
   da TCI, driver) rendeva NaN gli stati dei filtri e da li' non usciva piu' testo finche' non si cambiava modo. Ora vale zero, come nella
   cascata (`analyzer.rs:178`). Prova `un_campione_nan_non_blocca_il_ricevitore`. L'SSTV invece si riprende da sola (prova
   `un_campione_nan_non_blocca_la_ricezione`, passa anche prima): li' non si e' cambiato niente.
7. **Un panic nella decodifica fermava il motore** (`core/crates/engine-native/src/lib.rs`, ciclo dei periodi): decodifica e trasmissione
   girano nello stesso thread, lanciato con `std::thread::spawn` senza controllo; un difetto del decoder su un audio particolare lo uccideva e
   con lui l'audio verso SSTV e modi testuali (il passaggio `aurora-audio-tap` si chiude quando il motore non riceve piu'), fino al riavvio.
   Ora la decodifica del periodo e' dentro `guarded` (`catch_unwind`): si perde quel periodo, l'errore va nel log, il periodo dopo si
   decodifica. Prova `a_panicking_decode_skips_only_that_period`.

### Robustezza (2026-11-13, dopo la verifica; ogni punto con la sua prova)
8. **File salvati e `ALL.TXT` senza limite**: voce «Tieni i file salvati (giorni)» nella scheda Audio (`SaveKeepDays`, di base 7, 0 = per
   sempre). `engine_native::cleanup`, all'avvio e una volta al giorno: nella cartella dei salvataggi toglie solo i file `aammgg_hhmmss.wav`
   piu' vecchi del limite (eta' dal nome, niente sottocartelle); nella cartella dei dati ruota `AAAAMM_ALL.TXT` in `ALL-<data>.TXT` oltre
   50 MB o se cominciato prima del limite, e toglie i ruotati e i mesi finiti oltre il limite. Nessun altro file si tocca; riga nel registro
   con file tolti, MB liberati e rotazioni. Prove su cartelle temporanee in `cleanup.rs`.
9. **Thread del CAT morto per un panic** (Hamlib, TCI, FT-991(A) Link, Cat4OM): il guardiano del servizio CAT (`rig-control/src/watchdog.rs`
   e `cat.rs`) vede il thread finito per un panic, stacca la radio (PTT spento se si puo'), scrive un `warn` e ricollega dopo 1, 2, 4, 8,
   16 s (massimo 30); oltre 5 tentativi in 10 minuti resta in errore fino al collegamento a mano. In pagina lo stato e' «riconnessione».
   OmniRig si riprendeva gia' da solo. Prove con un collegamento finto che va in panic.
10. **Eventi persi sul bus**: ogni `RecvError::Lagged` ora scrive una riga (`aurora_core::lag::LagWatch`): `warn` al massimo ogni 60 s per
    ascoltatore, `error` a ogni perdita per chi scrive i QSO (`qso-export`, `QsoCompleted` -> `QsoLogged`, log del contest).
11. **Coda dell'audio verso il motore senza limite**: `audio_spectrum::queue`, tetto di 30 s di audio (quasi tre volte la decodifica piu'
    lunga, quindi nell'uso normale non si perde niente); oltre si buttano i blocchi piu' vecchi con un `warn` al massimo ogni 30 s. Cinque
    prove unitarie; le 220 prove di `engine-native` passano (il motore riceve da qualunque `ChunkSource`).
12. **Prove RTTY**: `6.283` sostituito da `TAU` in `tests/common/rtty_scenari.rs` (clippy `approx_constant`); la tabella dello scenario
    «scariche» e' identica prima e dopo (soglie -7,16 / -7,89 dB, CER medio 19,56).

## CW di tutta la banda più leggero (2026-11-13, ramo `lavoro/cw-banda-leggero`)
Regola: entra solo cio' che riduce la CPU in modo misurabile e non peggiora nessuna riga del banco CW (`cw_confronto`, decodificatore
probabilistico) di piu' di 0,3 punti, ne' i nominativi verificati con 10 e 30 stazioni. La prova `cpu_tutta_la_banda` ora stampa anche i
caratteri sbagliati dei canali (testo del canale piu' vicino a ogni stazione contro quello mandato), un controllo piu' fine dei nominativi.

**Dove va il tempo** (contatori di cicli messi per la misura e poi tolti, 30 canali): il traliccio del decodificatore probabilistico era
circa l'80% di un canale (espansione delle ipotesi con sopravvivenze e densita' delle durate circa il 30%, indice delle ipotesi circa il
15%, verosimiglianza dei toni circa il 12%, potatura circa il 7%); filtri circa il 7% (un resto di divisione per coefficiente); noise
blanker e AGC del rumore insieme solo il 5-8%. Per questo i guadagni sono venuti dal traliccio e non dal blanker e dall'AGC comuni.

### Prima e dopo (stessi bit: impronta dell'uscita, banco e prove di tutta la banda identici)
Misure alternate sullo stesso PC carico (altre compilazioni in corso; le percentuali assolute sono piu' alte di quelle della tabella sopra,
il rapporto e' quello che conta):

| Misura | Prima | Dopo |
|---|---|---|
| Tutta la banda CW, 10 stazioni (`cpu_tutta_la_banda`) | 8,9-9,2% di un core | 5,4% (-40%) |
| Tutta la banda CW, 30 stazioni | 26,1-28,1% di un core | 16,7-17,1% (-38%) |
| Nominativi verificati 10 / 30 stazioni, caratteri sbagliati | 10/10 e 23/30, 6,9% e 17,7% | identici |
| `tutta_la_banda` (10 stazioni) | 10/10 | 10/10 |
| CW, un ricevitore (`prestazioni`, fascio di 320) | 21,7-22,6 ms/s | 11,6-11,8 ms/s (-47%), impronta `609fc49e09fc0b1c` uguale |
| Banco `confronto_tasso_di_errore`, tutte le 11 righe | media 23,4 | identico casella per casella (media 23,4) |
| Un canale (30 canali, cicli del thread) | 30,2-30,9 Mcicli/s | 18,9-19,5 Mcicli/s |

### Entrato
1. **Sopravvivenze e densita' delle durate in memoria** (`cw_bayes.rs`, `Memo`): dipendono solo da durata, velocita' e pugno, e le stesse
   durate tornano a ogni passo per altre ipotesi della stessa velocita'. Tabella a indirizzo diretto (2048 posti), svuotata quando il pugno
   cambia; le soglie delle pause per velocita' si calcolano allo stesso momento. Un canale da circa 30,5 a circa 25 Mcicli/s.
2. **Indice delle ipotesi a indirizzamento aperto** con un timbro per passo al posto della `HashMap` (`HypIndex`).
3. **Filtri su un anello scritto due volte** (`cw.rs`): ogni finestra dei filtri (ricezione, strada larga, decodificatore, campioni
   cancellati e guadagni dell'AGC) e' una fetta contigua; prima ogni coefficiente (circa 1000 per campione decimato) costava un resto di
   divisione. Insieme a 1 e 2: da circa 30,5 a circa 24 Mcicli/s.
4. **Niente allocazioni a ogni passo del traliccio** (lo spazio delle ipotesi di prima passa ai candidati del passo dopo, il vettore delle
   migliori per velocita' si riusa) e radici delle cinque ampiezze calcolate una volta per tono invece che per ogni pezzo: da circa 24 a
   circa 19 Mcicli/s.

Tutte e quattro valgono anche per il ricevitore della pagina CW (stesso codice), che costa la meta'.

### Provato e scartato
| Idea | Risultato |
|---|---|
| **Noise blanker e AGC del rumore calcolati una volta sull'audio largo**, campioni cancellati e guadagno passati a tutti i canali (`SharedFront`, i canali mescolano il campione gia' ritardato) | CPU entro il rumore della misura (30 stazioni 647-668 contro 640-667 Mcicli/s, 10 stazioni 214-218 contro 222-233): blanker e AGC pesano il 5-8% di un canale, e cambiando di poco la decodifica cambia anche il lavoro del traliccio. Con 30 stazioni **22/30** nominativi invece di 23/30 e caratteri sbagliati dal 17,7% al **18,6%**: scartato |
| Solo il noise blanker comune (AGC di ogni canale) | 30 stazioni 24/30 e 17,3%, 10 stazioni uguale, ma la CPU non scende in modo misurabile (699-745 contro 640-667 Mcicli/s con 30 stazioni, 218-231 contro 222-233 con 10): non entra |
| Spettro dell'AFC dei canali calcolato una volta | non costruito: lo spettro dell'AFC e' l'1% di un canale (DFT su 53 righe, otto volte al secondo, gia' con le tabelle calcolate una volta) |
| Canali silenziosi aggiornati meno spesso | non costruito: saltare passi del traliccio cambia le durate delle pause (e quindi i risultati) alla ripresa del segnale, e nella prova le stazioni trasmettono tutto il minuto, quindi il guadagno non si vedrebbe; resta un'idea per la banda vera, da misurare con stazioni che ascoltano |
| Memoria delle durate da 4096 o 16384 posti invece di 2048 | nessuna differenza misurabile; 2048 occupa meno (32 kB per canale) |

## Problemi trovati e non corretti (con il motivo)

### Memoria e file che crescono
- *Chiuso (punto 8)*: **Salvataggio dei periodi «come JTDX» senza pulizia** (`engine-native/src/wav.rs:193`, `save_jtdx_style`): con «Salva tutti» un file ogni
  15 s, circa 360 kB, cioe' circa 2 GB al giorno. JTDX fa lo stesso e i file sono dell'utente: cancellarli e' una scelta del proprietario
  (proposta: un limite in GB o in giorni nelle impostazioni). Le registrazioni di misura (`save_slot`) tengono gia' gli ultimi 400 file.
- *Chiuso (punto 8)*: **`ALL.TXT`** (`engine-native/src/alltxt.rs`): un file al mese, mai cancellato (come JTDX); decine di MB a settimana su banda piena.
- **Storico delle decodifiche** (`decode-history`): pulizia oraria a 30 giorni, ma con `AURORA_HISTORY_DAYS=0` cresce senza limite e il
  file SQLite non si restringe mai (nessun `VACUUM`).
- *Chiuso (punto 11) per la coda verso il motore; quella fra la scheda e il passaggio resta un mpsc, ma il passaggio non si blocca mai
  (manda a SSTV e modi testuali con `try_send`)*: **Canali audio illimitati verso il motore** (`aurora/src/lib.rs:2679` e `:2681`, `std::sync::mpsc::channel`): il motore decodifica nello
  stesso thread che li svuota; durante una decodifica lunga (fino a 11 s, poi `deadline`) i blocchi si accumulano (circa 190 kB/s a 48 kHz)
  e poi si recuperano. Limitato dal tempo della decodifica, non dalla coda: se il thread si bloccasse (CAT, PTT appesi) crescerebbero
  senza fine. Non toccato: una coda limitata perderebbe audio del periodo.
- **Bus degli eventi e dello spettro a 4096 posti** (`aurora/src/lib.rs:2002`): un client WebSocket che resta indietro puo' tenere fino a
  4096 righe di spettro (~28 MB) finche' il timeout di 5 s dell'invio non lo chiude.
- Verificati con un limite: `log-in-attesa.json` (2000 voci, 7 giorni), spot del cluster (2000, 24 h), coda spot PSK (5000), coda degli
  invii (2000 per destinazione), cache QRZ/HamQTH (5000), immagini SSTV (200 ricevute, 20 trasmesse), `native-decodes.log` e
  `native-parity.log` (rotazione a 20 MB), elenchi del cervello delle automazioni (5000/100), liste interne del motore.

### Eventi che si possono perdere (`broadcast`, `RecvError::Lagged`)
*Avviso nel registro aggiunto (punto 10); gli eventi persi restano persi.*
Chi resta indietro sul bus perde gli eventi senza avviso: `QsoCompleted` -> `QsoLogged` (`aurora/src/lib.rs:829`), log e ADIF
(`qso-export/src/lib.rs:169`), invii ai servizi (`reporting/src/lib.rs:560`), contest (`contest/src/servizio.rs:1100`), automazione
(`qso-engine/src/runner.rs:121`), storico (`aurora/src/history.rs:60`). Con 4096 posti e pochi eventi al secondo serve un blocco di minuti,
quindi e' improbabile; non misurato in pratica, non toccato. Conviene almeno scrivere un avviso nel log (come fa gia' `wanted.rs:131`).
Il WebSocket (`remote-api/src/ws.rs:65`) dopo un `Lagged` rimanda solo lo stato: decodifiche e evidenziazioni di quel tratto mancano.

### Thread che, se muoiono, non ripartono
- *Chiuso (punto 9), tranne il rotore*: CAT (`rig-control/src/hamlib_rig.rs:827`, `tci.rs:869`, `cat4om.rs:1516`, `ft991link.rs:567`, `rotator.rs:463`): handle tenuto ma mai
  controllato; lo stato resta l'ultima istantanea.
- Ricezione di SSTV e modi testuali: `feed` ignora `Disconnected` (thread morto) mentre l'ascolto risulta acceso. Con le correzioni 5 e 6
  le due cause di panic trovate sono tolte.
- Il motore stesso (`engine-native/src/lib.rs:288`): ora protetta la decodifica, che e' la parte piu' esposta; il resto del ciclo no.

### Lock
- Lettura dei misuratori Hamlib con il lock `meter` tenuto durante l'I/O con rigctld (`hamlib_rig.rs:691`): il PTT aspetta dietro una
  lettura (fino a `io_timeout`). Ordine dei lock sempre lo stesso: nessun deadlock trovato nei crate esaminati.

### CPU che si puo' ancora togliere (cambia i calcoli: serve rifare i banchi di confronto)
- **CW di tutta la banda** (dopo il 2026-11-13 circa il 17% di un core con 30 stazioni, ancora la voce piu' pesante): blanker e AGC comuni
  provati e scartati (vedi «CW di tutta la banda più leggero»: pesano poco e cambiano i nominativi). Quello che resta e' il traliccio
  (fascio di 96 ipotesi per canale: con 32 un canale costa circa un terzo in meno, ma decodifica diversamente e va rimisurato) e i canali
  silenziosi, che oggi lavorano come gli altri.
- **CW, un ricevitore** (1,2% di un core): `sin_cos`, `log10` e `powf` per campione (`cw.rs`, `cw_bayes.rs`, `push_observed` e
  `observe_level`); la verosimiglianza dei toni (`ln_i0` per pezzo e ampiezza).
- **RTTY**: le catene Normalized e Limiter ricalcolano front-end e filtri della catena principale (`rtty.rs:1447-1487`, `1666-1686`).
- **PSK di tutta la banda**: un blanker e un `sin_cos` in f64 per campione per canale (`skim/psk.rs:230`, `psk.rs:426`).
- **Ricampionamento a 44,1 kHz** (1,9% di un core): servirebbe un filtro polifase, con campioni diversi (di poco) da quelli di oggi.
- **Cascata**: la FFT cresce con la frequenza della scheda (65536 punti a 48 kHz, 60211 a 44,1 kHz, la piu' lenta perche' non e' una potenza
  di 2) mentre servono solo i canali sotto 5 kHz: decimando prima a 12 kHz basterebbe una FFT da 16384, con righe leggermente diverse.
- **Tre ricampionamenti dello stesso audio** (motore, modi testuali, SSTV) e quattro-cinque copie di ogni blocco (`capture.rs:358`,
  `service.rs` di digi e SSTV, `band.rs:216`): un flusso unico a 12 kHz in `Arc<[f32]>` li ridurrebbe a uno.
- **SSTV**: a ogni gruppo di righe si clona l'immagine intera (`mode-sstv/src/service.rs:695`, fino a 1,5 MB).

## Cosa conviene fare dopo
1. CW di tutta la banda: blanker e AGC comuni, rimisurando `cpu_tutta_la_banda` e `cw_confronto` (la CPU scenderebbe di molto, ma i
   caratteri cambiano e vanno confrontati).
2. *Fatto (punti 9 e 10).* Resta il rotore (`rotator.rs:463`), che non riparte.
3. *Fatto (punto 8).*

1. CW di tutta la banda: fatto in parte (-37%, stessi bit, vedi sopra); il passo dopo e' rallentare i canali silenziosi, da misurare con
   stazioni che ascoltano (pause lunghe) e non solo con stazioni che chiamano CQ tutto il minuto.
2. Un avviso nel log a ogni `Lagged` dei consumatori dei QSO, e una ripartenza (o almeno un avviso) per i thread CAT morti.
3. Un limite (giorni o GB) per i periodi salvati «come JTDX» e per `ALL.TXT`, da decidere con il proprietario.
4. Flusso audio unico a 12 kHz condiviso fra motore, SSTV e modi testuali.

## Solo il modo in uso consuma, e modalita' «PC lento» (2026-11-13)

### Inventario: cosa gira con l'FT8 sulla pagina principale
Sempre (servono all'FT8 o alla stazione): cattura audio (`audio-spectrum/src/capture.rs:308`, priorita' tempo reale), passaggio
`aurora-audio-tap` (`aurora/src/lib.rs`, consegna i blocchi al motore), motore e decodifica (`engine-native/src/lib.rs`, thread di
decodifica `ft8/src/pool.rs` a priorita' sotto il normale), cascata, frequenza della radio ogni 250 ms e strumenti ogni 100 ms
(`aurora/src/lib.rs:2939`, `:2953`), riapertura dell'audio ogni 5 s se manca (`:3019`), automazioni (1 s), programmazione delle bande
(15 s, `:1387`), band hopping (5 s), storico (scrittura ogni 2 s, pulizia oraria, `aurora/src/history.rs:50`), desiderati (3 s,
`aurora/src/wanted.rs`), servizi online (30 s, `aurora/src/online.rs`), aggiornamenti (60 s, `aurora/src/updates.rs`), elenco LoTW (ogni
ora, `online-info/src/lotw.rs:153`), DX Cluster (connessione TCP), PSK Reporter (invio periodico, `reporting/src/lib.rs:513`), orologio
(15 min), rapporti (60 s).

Di altri modi, e cosa si fa adesso:

| Cosa | Dove | Prima | Adesso |
|---|---|---|---|
| Ascolto SSTV (copia, ricampionamento, decoder) | `mode-sstv/src/service.rs` (`feed`, `rx_loop`) | la pagina lo spegneva uscendo, ma una finestra staccata o un telefono che uscivano lo spegnevano anche agli altri, e una finestra chiusa senza smontare la pagina lo lasciava acceso per sempre | conteggio delle pagine con battito (`aurora_core::users`): si spegne quando esce l'ultima o dopo 90 s senza battiti, salvo «Ascolto acceso all'avvio» |
| Stato SSTV verso l'interfaccia | `mode-sstv/src/service.rs`, `rx_loop` | un evento `sstv rx_status` ogni 0,7 s a ogni client anche con l'ascolto spento | solo con l'ascolto acceso |
| Ascolto PSK · RTTY · CW e tutta la banda | `mode-digi/src/service.rs`, `band.rs` | come l'SSTV | come l'SSTV (tutta la banda riceve audio solo dall'ascolto) |
| Lettura del modo della radio per l'RTTY | `mode-digi/src/service.rs`, `rx_loop` | ogni secondo anche ad ascolto spento | solo in ascolto |
| Forme dei nominativi (log intero, 100 000 spot, MASTER.SCP) | `aurora/src/lib.rs`, `spawn_call_forms` | all'avvio e ogni 30 min, sempre | solo con tutta la banda CW/PSK al lavoro, al massimo ogni 30 min |
| Satelliti: posizione ogni secondo, passaggi delle 24 ore ogni minuto | `mode-sat/src/service.rs`, `tracking_loop` | sempre | solo con la pagina aperta (ogni `GET /sat` e' un battito valido 30 s) o con Doppler o rotore accesi |
| Dati delle orbite | `mode-sat/src/service.rs` | scaricati all'avvio se vecchi, poi ogni `tle_update_hours` | solo mentre il servizio lavora, ogni `tle_update_hours` (24 h di base) |
| Copie e ricampionamenti dell'audio per SSTV e modi testuali | `feed` dei due servizi | gia' nessuna copia ad ascolto spento | invariato (verificato) |
| QMAP | `qmap/src/service.rs` | lavora solo a richiesta (I/Q TCI) | invariato |

Con Doppler o rotore accesi i satelliti continuano a pagina chiusa: comandano radio e rotore, spegnerli a meta' passaggio romperebbe il QSO.

### Misura della CPU (processo del core fermo in FT8, senza segnale)
`aurora.exe` in release con una cartella dati nuova, ingresso il microfono della webcam (rumore di stanza: il motore decodifica ogni
periodo, 0 messaggi), nessuna pagina aperta; tempo di CPU del processo in 120 s dopo 45 s di riscaldamento. «Lasciati»: pagine SSTV e
PSK · RTTY · CW aperte e sparite senza salutare (battito e accensione, poi niente) e pagina dei satelliti letta una volta; la finestra di
misura comincia 100 s dopo. Script PowerShell `misura-cpu.ps1` (parametri `-Exe`, `-Etichetta`, `-Scenario fermo|lasciati`; non nel
repository). Durante le misure altri lavori compilavano sul PC: i numeri ballano di 1-2 punti.

| Scenario | Prima (2 giri) | Dopo (2 giri) |
|---|---|---|
| Fermo in FT8 | 9,3-11,1% di un core | 8,7-8,9% |
| Pagine SSTV e PSK · RTTY · CW lasciate accese | 12,3-15,5% (ascolto acceso per sempre) | 9,4-10,9% (ascolto spento dopo 90 s) |

### Modalita' «PC lento» (`aurora_core::idle`)
- **Porta**: il motore segna «decodifica in corso» per il lavoro di ogni periodo (`engine-native/src/lib.rs`, `idle::busy()`). Con la
  modalita' accesa i lavori pesanti di contorno aspettano la fine della decodifica, al massimo 30 s (`idle::MAX_WAIT`), poi partono
  comunque; i dati non si perdono (restano nel buffer TCP o in coda, oppure si chiedono dopo). Con la modalita' spenta la porta e'
  sempre aperta.
- **Chi la usa**: DX Cluster (lettura degli spot), PSK Reporter (invio periodico), servizi online (PSK Reporter in lettura, propagazione,
  DXpedition), ricerca dei nominativi QRZ/HamQTH, elenco LoTW, dati dei satelliti, forme dei nominativi, controllo degli aggiornamenti,
  pulizia oraria dello storico. Non la usano: la ricerca del nominativo al momento di registrare il QSO (ha gia' un limite di 2,5 s) e
  il flusso MQTT di PSK Reporter in tempo reale (deve rispondere ai ping del server; i pacchetti sono pochi e piccoli).
- **Quando si accende** (`Auto`, di fabbrica): 3 periodi su 10 oltre il 70% del periodo, oppure 3 blocchi audio persi in 10 minuti
  (`capture.rs`, errori del flusso con la scheda presente), oppure 2 processori o meno. **Si spegne** dopo 20 periodi di fila sotto il
  50% senza blocchi persi negli ultimi 10 minuti. Ogni cambio va nel registro (`aurora::pc_lento`).
- **Impostazione**: `SlowPcMode` (`auto`, `always`, `never`), Impostazioni › Modi › FT8 · FT4 › Decodifica, «PC lento», con il «?».

Misura (`cargo test -p ft8 --release --test misure -- --ignored --nocapture pc_lento`): PC simulato con 4 processori (affinita' del
processo), decodifica FT8 profonda con 2 thread sotto il normale come il motore, 4 lavori di contorno a priorita' normale sempre pronti,
periodo simulato pari a 2 volte la decodifica a PC scarico (713 ms, quindi 1426 ms; lento oltre 998 ms), 8 periodi, finto thread audio a
10 ms:

| Giro | Decodifica media / peggiore | Periodi lenti | Audio in ritardo (>30 ms) | Lavori di contorno | Attesa massima di un lavoro | Impronta |
|---|---|---|---|---|---|---|
| Contorno libero (modalita' spenta) | 1003 / 1174 ms | 4 su 8 | 0 | 124/s | 0 | `94c44064512265e1` |
| Contorno con la porta (modalita' accesa) | 819 / 1030 ms | 1 su 8 | 0 | 56/s | 1018 ms | = |
| Contorno libero a priorita' minima | 829 / 991 ms | 0 su 8 | 0 | 120/s | 0 | = |

I messaggi decodificati sono identici nei tre giri. Nella prova nessun blocco audio arriva in ritardo nemmeno senza porta (il thread
audio vero ha poi priorita' tempo reale): la porta serve soprattutto alla decodifica. **Priorita'**: abbassare i thread di contorno al
minimo da' lo stesso guadagno sulla decodifica senza rallentare il contorno, ma la maggior parte del contorno gira nei thread di Tokio,
gli stessi che servono l'interfaccia e il WebSocket: abbassarli rallenterebbe la pagina. Per questo non si e' cambiata nessuna priorita';
i thread dedicati (LoTW, dati delle orbite, forme dei nominativi) usano la porta.

### Non verificato
- Sul PC del proprietario con radio vera, finestre staccate e telefono: il conteggio delle pagine e' provato da prove automatiche
  (`l_ascolto_si_spegne_solo_quando_esce_l_ultima_pagina` in SSTV e modi testuali, `listenWhileOpen.test.tsx`), non a mano.
- Il rallentamento dei timer di WebView2 a finestra ridotta a icona: il battito ogni 20 s e la scadenza a 90 s lasciano margine anche
  con i timer a uno al minuto, ma non e' stato misurato.
- Un PC lento vero (2 processori): la modalita' si accende da sola, ma i numeri sono quelli della simulazione.
