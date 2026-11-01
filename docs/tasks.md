Elenco dei lavori presi da WSJT-X 3.x: ogni voce si cancella quando e' integrata in Aurora.

# Lavori da WSJT-X 3.x

Ordinati per priorita'. Analisi completa in `docs/analisi-wsjtx-3.md`. Sorgenti di riferimento: `D:\build\analisi-wsjtx\sorgenti`, tag `v3.3.0-beta5` (GPL-3, compatibile con Aurora GPL-3.0-only: citare la provenienza nei commenti e in `docs/licenze-terze-parti.md`).

## 1. SuperFox: codice QPC (127,50) su GF(128)

- Cosa fare: portare in Rust codifica e decodifica del Q-ary Polar Code e il CRC a 21 bit (`lookup3` `hashlittle`, seme 571, maschera `0x1FFFFF` sui primi 47 simboli).
- Aurora: nuovo crate `core/crates/superfox` (`qpc.rs`, `crc.rs`).
- WSJT-X: `lib/superfox/qpc/np_qpc.c`, `lib/superfox/qpc`, `qpc_fwht.c`, `qpc_subs.c`, `nhash2.c`, `lookup3.c`, `lib/superfox/qpc_decode2.f90` (controllo CRC).
- Verifica: test di andata e ritorno codifica/decodifica su messaggi casuali; CRC confrontato con valori calcolati da `nhash2` della 3.3.0-beta5 compilato a parte.

## 2. SuperFox: ricezione (sincronismo, demodulazione, decodifica)

- Cosa fare: catena `sfox_remove_ft8` -> `sfox_ana` -> `sfox_remove_tone` -> `qpc_sync` (24 simboli di sync, picco in `fsync +- ftol`) -> `sfox_demod` -> `qpc_likelihoods2` -> decodifica con dither (frequenza/tempo, smussamento, perturbazioni con semi fissi) -> scarto sotto -16,5 dB.
- Aurora: `core/crates/superfox` (`sync.rs`, `demod.rs`, `decode.rs`); riuso della sottrazione FT8 di `core/crates/ft8/src/subtract.rs`.
- WSJT-X: `lib/superfox/sfrx_sub.f90`, `sfox_mod.f90`, `qpc_sync.f90`, `sfox_demod.f90`, `qpc_likelihoods2.f90`, `qpc_snr.f90`, `qpc_decode2.f90`, `sfox_remove_ft8.f90`, `sfox_remove_tone.f90`, `sfox_ana.f90`, `twkfreq2.f90`.
- Verifica: file WAV generati con `sfoxsim` (da compilare dai sorgenti) a SNR diversi; Aurora deve decodificare almeno quanto `jt9.exe` 3.0.2 sugli stessi file e con lo stesso SNR, DT e frequenza.

## 3. SuperFox: unpack dei 329 bit e righe di decodifica

- Cosa fare: tipi `i3` 0, 2, 3 (1 si scarta come nella beta5), slot `QU1RKS` ignorati, una riga per Hound (`K1ABC FOX RR73`, `W9XYZ FOX -14`), `CQ FOX` se il bit q1 e' acceso, testo libero da 26 caratteri, riga `$VERIFY$ FOX 123456`; frequenza mostrata `fbest-750`, marcatore `~`.
- Aurora: `core/crates/superfox/src/message.rs`, riuso di c28/g15/t71 da `core/crates/ft8/src/message.rs`.
- WSJT-X: `lib/superfox/sfox_unpack.f90`, `superfox_qpc_2.txt`, `sfox_pack.f90` (per i test).
- Verifica: test con pacchetti costruiti a mano per ogni tipo e confronto con l'uscita di `sfox_unpack`.

## 4. SuperFox nel motore: Hound con SuperFox attivo

- Cosa fare: opzione «SuperFox» accanto a Hound; con tutte e due accese, nel periodo pari e a fine periodo si chiama il decodificatore SuperFox al posto dell'FT8; Rx forzata a 750 Hz (ripristino all'uscita), Rx limitata a 200-1400 Hz, FTol al massimo 100 Hz; niente limite di decodifica a 1000 Hz.
- Aurora: `core/crates/engine-native/src/lib.rs` (scelta del decodificatore e parita'), `engine-native/src/rig.rs` (interruttore `superfox`), `rig-control/src/controls.rs`, `app/src/components/RxControls.tsx`, stringhe in `app/src/i18n`.
- WSJT-X: `lib/ft8_engine_kernel.f90` (righe 232-236), `widgets/mainwindow.cpp` (5262-5271, 5396-5409, 11261-11280, 12180-12189, 13512-13515).
- Verifica: riproduzione di un WAV SuperFox dal motore proprio; le righe compaiono nella pagina Stazione; prova dal vivo del proprietario durante una DXpedition SuperFox.

## 5. Sequenza SuperHound

- Cosa fare: avvio del QSO solo con doppio clic su una riga SuperFox o con «Wait and Call» verso il nominativo del Fox; niente salti di frequenza (`hound_hop` spento), Tx a frequenza libera; il sequenziatore riconosce `MIOCALL FOX -14` (rapporto: si manda R+rapporto) e `MIOCALL FOX RR73` (registra e ferma); il QSO si registra in FT8; tx1/tx3 scritti a mano e griglia DX si svuotano.
- Aurora: `core/crates/qso-native/src/sequencer.rs`, `types.rs` (`Options::superhound`), `history.rs`, `core/crates/engine-native/src/tx.rs`.
- WSJT-X: `widgets/mainwindow.cpp` (`on_dxCallEntry_textChanged` 10414, `on_tx1_editingFinished` 10053, 7714-7735, 9939), `widgets/HoundTransmissionPolicy*`.
- Verifica: test in `core/crates/qso-native/tests/` con una sequenza SuperFox simulata (chiamata, rapporto, RR73, log); prova dal vivo.

## 6. Verifica del Fox con OTP (Hound classico e SuperHound)

- Cosa fare: riconoscere `CALL.123456` (Hound classico) e `$VERIFY$ CALL 123456` (SuperFox); se il codice non e' `000000` e la verifica in linea e' accesa, chiedere `GET {url}/check/{call}/{ora ISO}/{codice}.text` (predefinito `https://www.9dx.cc`, timeout 5 s); mostrare «CALL verified/invalid», etichetta Hound verde; opzione «Mostra messaggi OTP»; le righe OTP non vanno al server UDP.
- Aurora: `core/crates/engine-native` (riconoscimento), `core/crates/aurora/src/online.rs` (richiesta), impostazioni in `app/src/pages` e `app/src/i18n/strings/engineconfig.ts`, `udp-bridge/src/emit.rs` (filtro `$VERIFY$`).
- WSJT-X: `widgets/mainwindow.cpp` `processSFoxVerification` (17360-17415), `handleVerifyMsg`, `Network/FoxVerifier.cpp/.hpp`, `Configuration.cpp` (`OTPUrl`, `ShowOTP`).
- Verifica: test con risposta HTTP finta (verified/invalid/errore); nessuna richiesta con `000000` o con verifica spenta.

## 7. Cascata e spot per SuperFox

- Cosa fare: in SuperHound segnare sulla cascata la banda del SuperFox (circa 1500 Hz attorno a 750 Hz di sync) e la tolleranza; mandare a PSK Reporter gli spot dei decode SuperFox; righe SuperFox in ALL.TXT come WSJT-X.
- Aurora: cascata in `app/src`, `core/crates/reporting/src/psk.rs`, `core/crates/engine-native/src/alltxt.rs`.
- WSJT-X: `widgets/widegraph.cpp`, `widgets/plotter.cpp` (`setSuperFox`, `setSuperHound`), note 2.7.0-rc6.
- Verifica: confronto visivo con WSJT-X sullo stesso WAV; spot visibili su PSK Reporter.

## 8. Decodifica FT8 "a8" sul corrispondente

- Cosa fare: con DX Call e DX Grid noti, a Rx +-10 Hz provare le ipotesi di messaggio del QSO (circa 206) per correlazione sul segnale, come seconda possibilita' quando LDPC fallisce.
- Aurora: `core/crates/ft8/src/decode.rs`, nuovo `core/crates/ft8/src/a8.rs`.
- WSJT-X: `lib/ft8/ft8_a8d.f90`, `lib/ft8_decode.f90` (381-397).
- Verifica: WAV con il corrispondente a SNR -22..-24 dB: piu' decode del corrispondente senza aumentare i falsi (insieme di prova di `docs/analisi-decodium-decoder.md`).

## 9. Decodifica FT8 "a7" dal periodo precedente

- Cosa fare: salvare i decode di ogni parita' e, nel periodo successivo della stessa parita', cercare i messaggi attesi (seguito dei QSO osservati) nelle stesse frequenze/DT.
- Aurora: `core/crates/ft8/src/history.rs`, nuovo `a7.rs`, chiamata da `decode.rs`.
- WSJT-X: `lib/ft8/ft8_a7.f90` (`ft8_a7_save`, `ft8_a7d`), `lib/ft8_decode.f90` (175, 329-374).
- Verifica: sequenze di WAV consecutivi; contare decode aggiunti e falsi rispetto a oggi.

## 10. Decodifica FT8 multithread

- Cosa fare: dividere la banda in fasce e lanciarle in parallelo (rayon), con sottrazione coordinata dei segnali; scegliere il momento di avvio della decodifica.
- Aurora: `core/crates/ft8/src/decode.rs`, `subtract.rs`, `engine-native/src/lib.rs`.
- WSJT-X: `lib/ft8var/ft8_decodevar.f90`, `ft8_decode_ranges.f90`, `ft8_coarse_sync.f90`, `ft8_mtd_residual.f90`, `ft8_coarse_mutex.cpp`; `widgets/mainwindow.cpp` 11280-11290 (`m_ft8DecoderStart`).
- Verifica: stessi decode del singolo thread sui WAV di prova (a parita' di opzioni) e tempo di decodifica ridotto su banda satura.

## 11. UDP: TxInhibit e InhibitStatus (tipi 17 e 18)

- Cosa fare: ricevere `TxInhibit` (lease 100-30000 ms per controller, max 64, OR delle prenotazioni, TTL 0 rilascia), bloccare e riprendere il PTT; mandare `InhibitStatus` dopo ogni Heartbeat e a ogni cambio; solo con «Accetta richieste UDP».
- Aurora: `core/crates/udp-bridge/src/message.rs`, `request.rs`, `emit.rs`; `core/crates/tx-native/src/ptt.rs`.
- WSJT-X: `Network/NetworkMessage.hpp` (righe 549-641), `Network/MessageClient.cpp`, `widgets/mainwindow.cpp` (gestione della prenotazione).
- Verifica: test con un controller UDP finto: prenotazione, rinnovo, scadenza, rilascio, valori fuori limite ignorati e contati.

## 12. Wait and Reply / Wait and Call / Wait and Pounce

- Cosa fare: aspettare che il corrispondente sia libero (fine del suo QSO) prima di rispondere/chiamare, invece di trasmettere sopra.
- Aurora: `core/crates/qso-native/src/sequencer.rs`, `types.rs`, interfaccia.
- WSJT-X: `widgets/mainwindow.cpp` `processWaitReplyCall`, `WaitFeatureContext`, `wait_and_call_arming_eligible`.
- Verifica: test di sequenza con il DX occupato in un altro QSO.

## 13. Messaggi non rappresentabili bloccati prima del PTT

- Cosa fare: se un messaggio generato non si impacchetta esattamente (nominativo, prefisso, seriale, campo contest), non trasmettere e avvisare.
- Aurora: `core/crates/ft8/src/message.rs`, `core/crates/engine-native/src/tx.rs`.
- WSJT-X: `lib/ft8/genft8.f90` (`pack77_legacy_truncating_fallback_for_state`), `lib/ft4/genft4.f90`, note 3.2.
- Verifica: test con nominativi troppo lunghi e composti non standard.

## 14. SWR alto: stop della trasmissione

- Cosa fare: con SWR letto dalla radio sopra 2,5 fermare la Tx e avvisare; potenza e SWR in barra di stato.
- Aurora: `core/crates/rig-control/src/meters.rs`, `extras.rs`, `engine-native/src/tx.rs`.
- WSJT-X: note 3.0 («Transmitted Power and SWR»), `widgets/mainwindow.cpp`.
- Verifica: `fake_rigctld` con SWR 3,0 durante la Tx.

## 15. Rifiniture CAT dalla 3.2

- Cosa fare: mantenere DATA2/DATA3 scelti; ruoli dei VFO stabili in split; split riallineato al cambio dell'offset Tx; nessun cambio di banda durante Tx/Tune; scartare PTT/Tune pendenti dopo un errore della radio; rifiutare frequenze non valide.
- Aurora: `core/crates/rig-control/src/modes.rs`, `cat.rs`, `session.rs`, `engine-native/src/tx.rs`.
- WSJT-X: note 3.2 «Rig control and TCI», `Transceiver/HamlibTransceiver.cpp`, `widgets/mainwindow.cpp` (`requestNominalFrequencyChange`).
- Verifica: test con `fake_rigctld`; prova dal vivo sulla radio del proprietario.

## 16. TCI

- Cosa fare: controllo radio e audio via TCI (WebSocket) per radio SDR (Expert, SunSDR, ANAN con Thetis).
- Aurora: `core/crates/rig-control` (nuovo `tci.rs`), `audio-spectrum` (ingresso audio TCI), `tx-native` (uscita audio).
- WSJT-X: `Transceiver/TCITransceiver.cpp/.hpp`.
- Verifica: con un server TCI di prova; dal vivo con una SDR.

## 17. Band Hopping FT8/FT4

- Cosa fare: periodi di ricezione a rotazione sulle sotto-bande o su fino a 8 frequenze scelte; clic destro per la frequenza successiva.
- Aurora: `core/crates/rig-control/src/schedule.rs`, interfaccia.
- WSJT-X: `widgets/mainwindow_bandhopping.cpp`.
- Verifica: test di pianificazione con orologio finto.

## 18. Controlli dalla 3.2 su PSK Reporter, ALL.TXT, Wavelog, ADIF

- Cosa fare: pacchetti PSK Reporter piu' piccoli e data giusta a mezzanotte; banda/frequenza del periodo in ALL.TXT e negli spot dopo un cambio; verifica della connessione Wavelog con scelta della stazione dal server; scambi contest nell'ADIF; `ADIF_VER` aggiornato.
- Aurora: `core/crates/reporting/src/psk.rs`, `wavelog.rs`, `engine-native/src/alltxt.rs`, `core/crates/logbook/src/adif.rs`.
- WSJT-X: note 3.2 «Networking, logging, and integrations», `Network/PSKReporter.cpp`, `logbook/`.
- Verifica: test unitari sui casi di mezzanotte e cambio banda.

## 19. Cascata: storia conservata al ridimensionamento

- Cosa fare: ridimensionare la cascata senza perdere le righe; appiattimento calcolato sul campo visibile.
- Aurora: cascata in `app/src`, `core/crates/audio-spectrum/src/analyzer.rs`.
- WSJT-X: `widgets/plotter.cpp`, `widgets/widegraph.cpp`, `widgets/WaterfallScale.hpp`.
- Verifica: prova manuale nell'interfaccia.

## 20. Modo Fox e SuperFox trasmettitore

- Cosa fare: Fox classico (fino a 5 slot, `foxgen`) e SuperFox (pack 329 bit, QPC, GFSK a 750 Hz), coda dei Hound con ordinamento UDP `AnnotationInfo` (tipo 16) e OTP con seme TOTP.
- Aurora: `core/crates/superfox`, `tx-native`, `engine-native/src/tx.rs`, `udp-bridge/src/message.rs`.
- WSJT-X: `lib/superfox/sfox_pack.f90`, `sftx_sub.f90`, `sfox_gen_gfsk.f90`, `sfox_wave_gfsk.f90`, `lib/ft8/foxgen.f90`, `widgets/SuperFoxTxPlanner.cpp`, `FoxGuardBands.cpp`, `FoxLogWindow.cpp`, `mainwindow.cpp` (`foxOTPcode`, `sfox_tx`).
- Verifica: il segnale generato si decodifica con `jt9.exe` 3.0.2 e con il decodificatore di Aurora.

## 21. Modo Q65

- Cosa fare: aggiungere Q65 (sottomodi A-E, periodi 15/30/60/120/300 s) con codifica, modulazione e decodifica, compresa la media dei messaggi e la decodifica con le ipotesi del QSO (AP).
- Aurora: nuovo crate `core/crates/q65`; scelta del modo in `engine-native`, `qso-native`, `app/src/components/ModeSwitch.tsx`.
- WSJT-X: `lib/qra/q65/` (`q65.c`, `q65.f90`, `q65_decode.f90`, `q65_sync.f90`), `lib/q65params.f90`, `widgets/mainwindow.cpp` (sezioni Q65).
- Verifica: WAV generati con `q65sim` a vari SNR; Aurora decodifica almeno quanto `jt9.exe`.

## 22. Modo MSK144 (meteor scatter)

- Cosa fare: aggiungere MSK144 (periodi 15/30 s, messaggi brevi «Sh»), decodifica dei ping brevi a blocchi nel periodo, sequenza dei QSO del meteor scatter.
- Aurora: nuovo crate `core/crates/msk144`; `engine-native`, `qso-native`, `ModeSwitch.tsx`.
- WSJT-X: `lib/msk144*.f90`, `lib/mskrtd.f90`, `lib/msk144decodeframe.f90`, `widgets/mainwindow.cpp` (MSK144).
- Verifica: WAV con ping simulati (`msk144sim`), confronto con `jt9.exe`.

## 23. Pulsanti rapidi dei modi

- Cosa fare: estendere i pulsanti di scelta del modo della pagina Stazione a Q65 e MSK144 quando ci saranno (oggi FT8, FT4, JT65, JT9).
- Aurora: `app/src/components/ModeSwitch.tsx`, `core/crates/rig-control/src/table.rs` (frequenze per modo).
- WSJT-X: `widgets/mainwindow.ui` (pulsanti dei modi della 3.0).
- Verifica: cambio modo dal pulsante con radio collegata.

## 24. Filtri dei decode per prefisso, nominativo e «gia' collegato»

- Cosa fare: confrontare con i filtri di WSJT-X 3.0 (nascondere o evidenziare per prefisso, nominativo, Worked Before) e aggiungere quelli che mancano in Aurora (oggi ha filtri e colori per DXCC, desiderati e gia' collegati).
- Aurora: `app/src/components/DecodeList.tsx`, `app/src/settings/FiltersTab.tsx`.
- WSJT-X: `widgets/mainwindow.cpp`, `Configuration.cpp` (filtri e evidenziazioni della 3.0).
- Verifica: decode di prova con prefissi e nominativi filtrati.

## 25. Full duplex per i satelliti

- Cosa fare: ricevere e decodificare mentre si trasmette quando la radio lo consente (satelliti), senza bloccare la ricezione durante il PTT.
- Aurora: `core/crates/engine-native/src/lib.rs` (oggi la decodifica si ferma in trasmissione), `tx-native`.
- WSJT-X: `widgets/mainwindow.cpp` (opzione Full Duplex), `Configuration.cpp`.
- Verifica: con radio satellitare in full duplex, decode del proprio segnale di ritorno.

## 26. QMAP: ricezione panoramica Q65 fino a 90 kHz

- Cosa fare: dopo il Q65 (voce 21), un pannello che decodifica tutta una fetta di banda larga fino a 90 kHz e con un clic imposta frequenza e parametri del QSO. Serve un ricevitore SDR a banda larga (flusso I/Q), non la sola scheda audio.
- Aurora: nuovo ingresso I/Q in `core/crates/audio-*`, decodifica Q65 su piu' canali, pagina nell'interfaccia.
- WSJT-X: cartella `qmap/`.
- Verifica: registrazione I/Q di prova con piu' segnali Q65.

## 27. Hamlib aggiornato

- Cosa fare: allineare la versione di Hamlib usata da Aurora a quella di WSJT-X 3.x per il controllo CAT delle radio recenti.
- Aurora: `core/crates/rig-control` (libreria Hamlib inclusa e `docs/cat-radio.md`).
- WSJT-X: versione di Hamlib indicata in `CMakeLists.txt` e nelle note di rilascio.
- Verifica: elenco radio supportate e prova CAT del proprietario.
