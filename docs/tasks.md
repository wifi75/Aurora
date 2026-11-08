Elenco dei lavori presi da WSJT-X 3.x: ogni voce si cancella quando e' integrata in Aurora.

# Lavori da WSJT-X 3.x

Ordinati per priorita'. Analisi completa in `docs/analisi-wsjtx-3.md`. Sorgenti di riferimento: `D:\build\analisi-wsjtx\sorgenti`, tag `v3.3.0-beta5` (GPL-3, compatibile con Aurora GPL-3.0-only: citare la provenienza nei commenti e in `docs/licenze-terze-parti.md`).

**UltraFOX (0.11.70)**: le voci SuperFox 1-5 e 7 sono fatte come UltraFOX, compatibile con SuperFox, e sono state tolte; della 6 resta solo la parte qui sotto. Regola: ogni voce finita si toglie da questo file.

## 6. Verifica del Fox con OTP: parte SuperFox

- Fatto (Hound classico): riconoscimento di `CALL.123456`, righe nascoste con «Mostra i messaggi OTP» spento o codice `000000`, richiesta `GET {url}/check/{call}/{ora ISO}/{codice}.text` (5 s) solo con verifica in linea accesa e codice diverso da `000000`, esito nella pagina Stazione e riga «CALL verified/invalid» (`engine-native/src/features.rs`, `online-info/src/foxcheck.rs`, `aurora/src/lib.rs` `wire_fox_verifier`).
- Manca: il formato SuperFox `$VERIFY$ CALL 123456` (hz 750) e il filtro `$VERIFY$` verso il server UDP (`udp-bridge/src/emit.rs`): dipende dal ricevitore SuperFox (voci 1-3); l'etichetta Hound/SuperHound verde dell'interfaccia.
- WSJT-X: `widgets/mainwindow.cpp` `processSFoxVerification` (17360-17415), `handleVerifyMsg`.

## 10. Decodifica FT8 multithread: misura su banda satura

- Fatto: decodifica multithread (`mtd`) e momento di avvio della decodifica (`FT8DecoderStart`: Presto 13,82 s, Normale 14,11 s, Tardi 14,40 s, come `m_ft8DecoderStart`/`fixStop` di WSJT-X) in `engine-native/src/features.rs` e `slots.rs`.
- Fatto (2026-11-08): misura su 5 periodi veri della stazione con 42-46 messaggi (10 m), `ft8/examples/misura_wav.rs`: profondita' 2 in 0,7-0,85 s, profondita' 3 in 1,7 s (8 thread) - 2,8 s (1 thread), 192 messaggi contro 167. Non serve dividere la banda in fette su piu' processori.
- Manca: le decodifiche anticipate a due/tre passate di WSJT-X (`m_earlyDecode`, `m_earlyDecode2` a 46-47): una prima decodifica a circa 12 s sul periodo non finito, poi il completamento a fine periodo.

## 16. TCI (parziale)

- Fatto (2026-11-08, ramo `lavoro/tci`): backend CAT «TCI» (`rig-control/src/tci.rs`, `cat_config.rs` `Backend::Tci` e `tci_address`, base `127.0.0.1:40001`; `cat.rs` lo instrada). Protocollo letto da WSJT-X 3 (`TCITransceiver.cpp`, `TCIStream.hpp`, `TCIProtocol.hpp`): lettura dei comandi di testo (`vfo`, `modulation` anche nella forma a tre argomenti di ExpertSDR3/Thetis, `trx`, `protocol`, `device`, `start`/`stop`, `audio_start`/`audio_stop`, `audio_samplerate`, `ready`) e costruzione (`vfo:0,0,<hz>;`, `modulation:0,<usb|digu>;`, `trx:0,<bool>[,tci];`, `audio_start/stop:0;`, `audio_samplerate:`); intestazione binaria di 64 byte (ricevitore, frequenza, formato 3 = float32, codec, crc, lunghezza, tipo 0 IQ / 1 audio Rx / 2 audio Tx / 3 chrono, canali dalla 1.9) con lettura, scrittura, controllo del chrono e risposta audio Tx; 11 prove, anche con un server TCI finto. `TciRig` su `tungstenite` (thread, ricollegamento con attesa crescente, collegata solo dopo `ready;`): frequenza, modo USB/USB dati del «Modo CAT», PTT con `trx` (anche dal motore: `aurora/src/lib.rs` `cat_ptt`). Ganci per l'audio pronti: `set_rx_audio` (manda `audio_start` e consegna mono alla frequenza del server) e `set_tx_audio` (risponde ai chrono). Scheda Radio: scelta «TCI» con il campo indirizzo, 6 lingue; lo split e' nascosto con TCI.
- Resta:
  - audio ricevuto via TCI nel motore: una sorgente in `audio-spectrum` (accanto a `capture.rs`) che usa `TciRig::set_rx_audio`, porta i 48 kHz a 12 kHz (filtro e decimazione, vedi `engine-native/src/resample.rs`) e prende il posto del dispositivo di ingresso quando nella scheda Audio si sceglie «TCI»; anche analizzatore e livello d'ingresso;
  - audio da trasmettere via TCI: un `tx_native::AudioSink` che consegna l'onda (a 48 kHz) a `TciRig::set_tx_audio` e la fa uscire un chrono alla volta, con `is_playing`/`stop`; il PTT allora manda `trx:0,true,tci;` (gia' fatto in `set_ptt` quando il gancio Tx c'e');
  - scelta del ricevitore (oggi sempre 0; WSJT-X sa usare anche RX2 con `rx_enable:1,true;`), split con il VFO 1 (`vfo:0,1,<hz>;`, `split_enable`), misuratori (`rx_smeter`, `rx_sensors`/`tx_sensors`, `tx_power`, `tx_swr`), `drive`/`tune`, accensione con `start;`/`stop;`;
  - conferma dei comandi come WSJT-X (attesa dell'eco di `vfo`/`trx`): oggi si scrive e si aggiorna lo stato subito, poi l'eco del server lo corregge;
  - il controllo «sta trasmettendo» dell'API (`remote-api/src/cat.rs`) guarda solo Hamlib.
- WSJT-X: `Transceiver/TCITransceiver.cpp/.hpp`, `TCIStream.hpp`, `TCIProtocol.hpp`, prove in `tests/unit/tci`.
- Verifica: dal vivo con una SDR (ExpertSDR3 o Thetis con TCI attivo): collegamento, cambio di banda, modo, PTT.

## 18. Controlli dalla 3.2 su PSK Reporter, ALL.TXT, Wavelog, ADIF (parziale)

- Fatto (2026-11-01, ramo `wsjtx-cat`): pacchetti PSK Reporter entro 952 byte; ora dello spot = periodo del messaggio con la data giusta a mezzanotte; messaggi dei periodi prima di un cambio di banda/modo segnalati con la banda di allora; prova Wavelog/Cloudlog con permesso di scrittura (`api/auth`) ed elenco delle stazioni lette dal server nel messaggio della prova; `ADIF_VER` 3.1.8 (export ed eQSL); funzione `logbook::adif::contest_fields` con le regole di `AdifQso.cpp`.
- Resta: tendina della stazione Wavelog nelle Impostazioni (oggi le stazioni del server compaiono nel messaggio della prova); lo scambio di contest inviato (`exch_sent`) resta vuoto finche' il motore non fa la sequenza dei contest (lo scambio ricevuto e i campi ADIF sono fatti).
- WSJT-X: note 3.2 «Networking, logging, and integrations», `logbook/AdifQso.cpp`.
- Verifica: test unitari sui casi di mezzanotte e cambio banda.

## 20. Modo Fox e SuperFox trasmettitore

- Cosa fare: Fox classico (fino a 5 slot, `foxgen`) e SuperFox (pack 329 bit, QPC, GFSK a 750 Hz), coda dei Hound con ordinamento UDP `AnnotationInfo` (tipo 16) e OTP con seme TOTP.
- Aurora: `core/crates/superfox`, `tx-native`, `engine-native/src/tx.rs`, `udp-bridge/src/message.rs`.
- WSJT-X: `lib/superfox/sfox_pack.f90`, `sftx_sub.f90`, `sfox_gen_gfsk.f90`, `sfox_wave_gfsk.f90`, `lib/ft8/foxgen.f90`, `widgets/SuperFoxTxPlanner.cpp`, `FoxGuardBands.cpp`, `FoxLogWindow.cpp`, `mainwindow.cpp` (`foxOTPcode`, `sfox_tx`).
- Verifica: il segnale generato si decodifica con `jt9.exe` 3.0.2 e con il decodificatore di Aurora.
- Interfaccia approvata dal proprietario il 2026-11-01: `docs/mockup-fox.html` (contatori con grafico del ritmo, Hound con doppio ordinamento e filtri, posti del SuperFox con avanzamento e anteprima del pacchetto, parametri visibili, coda, Fox log con ADIF, Cabrillo e ClubLog). Limiti di WSJT-X da superare in `docs/analisi-wsjtx-3.md`.

## 21. Modo Q65 (parziale)

- Fatto (0.11.195): Q65-30A in `core/crates/jt/src/q65.rs` (codice QRA 15,65 su GF(64), CRC-12, messaggi a 77 bit), `Mode::Q65` nel motore, selettore dei modi e frequenze di WSJT-X (50,275 e 144,180 MHz, EME come alternative).
- Resta: scelta del periodo (15/30/60/120/300 s) e del sottomodo (A-E), media dei messaggi fra periodi, decodifica con le ipotesi del QSO (AP), modello di evanescenza per EME.
- WSJT-X: `lib/qra/q65/` (`q65.c`, `q65.f90`, `q65_decode.f90`, `q65_sync.f90`), `lib/q65params.f90`, `widgets/mainwindow.cpp` (sezioni Q65).
- Verifica: WAV generati con `q65sim` a vari SNR; Aurora decodifica almeno quanto `jt9.exe`.

## 23. Pulsanti rapidi dei modi

- Fatto (0.11.194-0.11.195): selettore dei modi con JT4 (sottomodi A-G), Q65 e MSK144, colori propri per modo.
- Resta: i periodi e i sottomodi di Q65 nel selettore, quando ci saranno (voce 21).
- Aurora: `app/src/components/ModeSwitch.tsx`, `core/crates/rig-control/src/table.rs` (frequenze per modo).
- Verifica: cambio modo dal selettore con radio collegata.

## 26. QMAP: ricezione panoramica Q65 fino a 90 kHz

- Cosa fare: dopo il Q65 (voce 21), un pannello che decodifica tutta una fetta di banda larga fino a 90 kHz e con un clic imposta frequenza e parametri del QSO. Serve un ricevitore SDR a banda larga (flusso I/Q), non la sola scheda audio.
- Aurora: nuovo ingresso I/Q in `core/crates/audio-*`, decodifica Q65 su piu' canali, pagina nell'interfaccia.
- WSJT-X: cartella `qmap/`.
- Verifica: registrazione I/Q di prova con piu' segnali Q65.
