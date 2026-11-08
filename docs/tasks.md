Elenco dei lavori presi da WSJT-X 3.x: ogni voce si cancella quando e' integrata in Aurora.

# Lavori da WSJT-X 3.x

Ordinati per priorita'. Analisi completa in `docs/analisi-wsjtx-3.md`. Sorgenti di riferimento: `D:\build\analisi-wsjtx\sorgenti`, tag `v3.3.0-beta5` (GPL-3, compatibile con Aurora GPL-3.0-only: citare la provenienza nei commenti e in `docs/licenze-terze-parti.md`).

**UltraFOX (0.11.70)**: le voci SuperFox 1-5 e 7 sono fatte come UltraFOX, compatibile con SuperFox, e sono state tolte; della 6 resta solo la parte qui sotto. Regola: ogni voce finita si toglie da questo file.

## 6. Verifica del Fox con OTP: parte SuperFox

- Fatto (Hound classico): riconoscimento di `CALL.123456`, righe nascoste con «Mostra i messaggi OTP» spento o codice `000000`, richiesta `GET {url}/check/{call}/{ora ISO}/{codice}.text` (5 s) solo con verifica in linea accesa e codice diverso da `000000`, esito nella pagina Stazione e riga «CALL verified/invalid» (`engine-native/src/features.rs`, `online-info/src/foxcheck.rs`, `aurora/src/lib.rs` `wire_fox_verifier`).
- Manca: il formato SuperFox `$VERIFY$ CALL 123456` (hz 750) e il filtro `$VERIFY$` verso il server UDP (`udp-bridge/src/emit.rs`): dipende dal ricevitore SuperFox (voci 1-3); l'etichetta Hound/SuperHound verde dell'interfaccia.
- WSJT-X: `widgets/mainwindow.cpp` `processSFoxVerification` (17360-17415), `handleVerifyMsg`.

## 16. TCI: verifica dal vivo

- Fatto (2026-11-08, ramo `lavoro/tci`): backend CAT «TCI» (`rig-control/src/tci.rs`, protocollo letto da WSJT-X 3), frequenza, modo, PTT, split vero sul VFO 1 (`split_enable`) e simulato, misuratori (`rx_sensors`/`tx_sensors`/`rx_smeter`/`tx_power`/`tx_swr`), `drive`/`tune` (metodi di `TciRig`, non ancora sui comandi dell'interfaccia), `start;`/`stop;` con «Accendi»/«Spegni»; audio ricevuto nel motore (`audio-spectrum/src/tci_source.rs`, ingresso «TCI (dalla radio)») e audio trasmesso un chrono alla volta (`tx-native/src/tci_sink.rs`, uscita «TCI (alla radio)»), ganci tenuti da `CatService` per ogni radio TCI collegata; «sta trasmettendo» dell'API anche con TCI. Guida: «SDR con TCI».
- Resta: verifica dal vivo con ExpertSDR3/Thetis (collegamento, cambio di banda, modo, PTT, audio nei due sensi, misuratori, accensione). Da valutare dopo la prova: scelta del ricevitore RX2, conferma dei comandi con l'eco come WSJT-X, `audio_samplerate` diverso da 48 kHz in trasmissione; lo split (come con Hamlib) non e' ancora comandato dal motore e resta nascosto nella scheda Radio.

## 18. Controlli dalla 3.2 su PSK Reporter, ALL.TXT, Wavelog, ADIF (parziale)

- Fatto (2026-11-01, ramo `wsjtx-cat`): pacchetti PSK Reporter entro 952 byte; ora dello spot = periodo del messaggio con la data giusta a mezzanotte; messaggi dei periodi prima di un cambio di banda/modo segnalati con la banda di allora; prova Wavelog/Cloudlog con permesso di scrittura (`api/auth`) ed elenco delle stazioni lette dal server nel messaggio della prova; `ADIF_VER` 3.1.8 (export ed eQSL); funzione `logbook::adif::contest_fields` con le regole di `AdifQso.cpp`.
- Fatto anche: tendina della stazione Wavelog/Cloudlog nelle Impostazioni (`settings/WavelogStationSelect.tsx`, letta dal server con `POST /api/v1/qso-upload/wavelog-stations`; senza risposta resta la casella del numero).
- Resta: lo scambio di contest inviato (`exch_sent`) resta vuoto finche' il motore non fa la sequenza dei contest (lo scambio ricevuto e i campi ADIF sono fatti): dipende da un lavoro che non c'e' ancora, non da questa voce.
- WSJT-X: note 3.2 «Networking, logging, and integrations», `logbook/AdifQso.cpp`.
- Verifica: test unitari sui casi di mezzanotte e cambio banda.

## 20. Modo Fox e SuperFox trasmettitore

- Cosa fare: Fox classico (fino a 5 slot, `foxgen`) e SuperFox (pack 329 bit, QPC, GFSK a 750 Hz), coda dei Hound con ordinamento UDP `AnnotationInfo` (tipo 16) e OTP con seme TOTP.
- Aurora: `core/crates/superfox`, `tx-native`, `engine-native/src/tx.rs`, `udp-bridge/src/message.rs`.
- WSJT-X: `lib/superfox/sfox_pack.f90`, `sftx_sub.f90`, `sfox_gen_gfsk.f90`, `sfox_wave_gfsk.f90`, `lib/ft8/foxgen.f90`, `widgets/SuperFoxTxPlanner.cpp`, `FoxGuardBands.cpp`, `FoxLogWindow.cpp`, `mainwindow.cpp` (`foxOTPcode`, `sfox_tx`).
- Verifica: il segnale generato si decodifica con `jt9.exe` 3.0.2 e con il decodificatore di Aurora.
- Interfaccia approvata dal proprietario il 2026-11-01: `docs/mockup-fox.html` (contatori con grafico del ritmo, Hound con doppio ordinamento e filtri, posti del SuperFox con avanzamento e anteprima del pacchetto, parametri visibili, coda, Fox log con ADIF, Cabrillo e ClubLog). Limiti di WSJT-X da superare in `docs/analisi-wsjtx-3.md`.

## 21. Modo Q65: verifica con WSJT-X

- Fatto (0.11.195 e dopo): Q65 completo nel motore proprio. Periodi 15/30/60/120/300 s e sottomodi A-E in trasmissione e ricezione (`core/crates/jt/src/q65.rs`), ipotesi AP del QSO (CQ, mio nominativo, con il DX, RRR/73/RR73, `q65_ap.f90`), metrica dell'evanescenza rapida con le tabelle gaussiane e lorentziane (`q65fading.rs`, `q65_intrinsics_fastfading`) provata dopo Bessel su sette larghezze B90, media dei periodi della stessa parita'. Periodo e sottomodo scelti accanto al selettore dei modi («Q65-60B»), salvati in `controls.json`; durata dei periodi, avvio e finestra di trasmissione e guardia seguono il Q65 scelto. Pulsanti rapidi dei modi (ex voce 23) completi.
- Fatto anche (2026-11-08): scelta del modello di evanescenza (Lor/Gau) e «Cancella media» accanto a periodo e sottomodo (interruttore `q65_gauss` ricordato in `controls.json`, pulsante `q65_clear_avg`); primo confronto con `jt9.exe` 3.2.0-rc1 su WAV di `q65sim` in `docs/q65-confronto.md` (esempio `core/crates/jt/examples/misura_q65.rs`): pari con Doppler e nei periodi lunghi, indietro di 1-2 dB senza Doppler in 15A/30A/60A, e molto piu' lento.
- Resta: recuperare quei 1-2 dB (ricerca del sincronismo e soglie come `q65_decode.f90`), trovare le soglie vere di `jt9` con piu' file per punto, confronto della media, velocita' della decodifica (dettagli in `docs/q65-confronto.md`).
- WSJT-X: `lib/qra/q65/` (`q65.c`, `q65.f90`, `q65_ap.f90`, `fadengauss.c`, `fadenlorentz.c`), `lib/q65_decode.f90`.
- Verifica: WAV di `q65sim` per ogni periodo e sottomodo; QSO dal vivo in Q65-60A su 6 m o 2 m.

## 26. QMAP: ricezione panoramica Q65 fino a 90 kHz (parziale)

- Fatto (2026-11-08, ramo `lavoro/qmap`): crate `core/crates/qmap`. Sorgente I/Q (`iq.rs`: WAV I/Q a due canali PCM 16/24/32 bit o float, centro letto dal nome `..._144100kHz.wav`; `tci_iq.rs`: collegamento TCI suo, separato da quello del CAT, `iq_samplerate`/`iq_start`/`iq_stop`, centro da `dds:`, blocchi binari di tipo 0 int16/int24/int32/float32). Ricerca (`search.rs`, da `qmapa.f90`, `getcand2.f90`, `q65b.f90`): FFT lunga quanto il periodo, spettro medio a 2,93 Hz normalizzato su 16 tratti (40esimo percentile), picchi del tono di sincronismo presi dal piu' forte con la larghezza del segnale azzerata, ogni candidato riportato a 12 kHz con il tono 0 a 1000 Hz e decodificato da `jt::q65` (piu' sottomodi, AP, evanescenza a scelta). Servizio (`service.rs`): periodo raccolto sull'orologio, ricerca in un thread, configurazione in `qmap.json`. API: `GET/PUT /api/v1/qmap`, `POST /api/v1/qmap/decode` (file), `POST /api/v1/qmap/tune` (Q65 con periodo e sottomodo del messaggio, radio con il tono 0 a 1000 Hz, Rx a 1000 Hz); la configurazione si cambia solo dal PC. Pagina `#/qmap` (link «QMAP» nel menu): sorgente, periodo, sottomodi, spettro della fetta con i segni cliccabili, elenco con «QSO»; 6 lingue. Prove: tre segnali Q65-15A a -31, +4 e +39 kHz in 96 kHz, 60A e 60B insieme, solo rumore, WAV, TCI con server finto.
- Resta: i periodi da 30 s dentro il minuto di QMAP (`bAlso30`, `iseq`) e la decodifica a meta' periodo; la media dei periodi; il sincronismo per candidato prima del decodificatore (`q65_sync`); Doppler EME (`astro`); prototipo grafico da approvare.
- Verifica dal vivo: con un SDR TCI (ExpertSDR/Thetis) a 96 kHz su 2 m o 6 m, e una registrazione I/Q vera con piu' segnali Q65.
- WSJT-X: cartella `qmap/` (`libqmap/qmapa.f90`, `getcand2.f90`, `q65b.f90`), `Transceiver/TCITransceiver.cpp`.
